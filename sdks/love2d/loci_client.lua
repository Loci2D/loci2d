local socket = require("socket")
local pb = nil
local protoc = nil

-- Attempt to load lua-protobuf library and protoc parser
local ok_pb, res_pb = pcall(require, "pb")
if ok_pb then
    pb = res_pb
    local ok_protoc, res_protoc = pcall(require, "protoc")
    if ok_protoc then
        protoc = res_protoc
    end
else
    print("[Warning] 'lua-protobuf' module not found.")
    print("Install lua-protobuf (e.g. via luarocks install lua-protobuf) to run full binary Protobuf encoding.")
end

local loci = {
    -- State
    entities = {},
    globals = {},
    my_entity_id = nil,

    -- Callbacks
    on_entity_spawned = function(entity) end,
    on_entity_despawned = function(entity_id) end,
    on_property_changed = function(entity, key, old_val, new_val) end,
    on_match_state_changed = function(state) end, -- Reserved for future use: Not yet broadcast by server
    on_action_cast = function(entity, ability_id, dir_x, dir_y) end, -- Reserved for future use: Action broadcasts not yet implemented
    on_intent_rejected = function(reason) end,
    
    -- Internal
    _udp = nil,
    _schema_loaded = false,
    _sequence_id = 0,
    _last_heartbeat_time = 0,
    _player_name = nil,
    _base_path = "lib/",
}

local function bits_to_float(bits)
    if not bits then return 0.0 end
    return bits / 65536.0
end

local function float_to_bits(f)
    if not f then return 0 end
    return math.floor(f * 65536)
end

function loci.connect(host, port, player_name, base_path)
    if base_path then
        loci._base_path = base_path
    end

    loci._udp = socket.udp()
    loci._udp:settimeout(0)
    loci._udp:setpeername(host, port)
    loci._player_name = player_name
    
    if pb then
        local schema_loaded = false
        -- 1. Try dynamic text parsing with protoc.lua
        if protoc then
            local p = protoc.new()
            p.include_dirs = { loci._base_path .. "../../proto", "proto", ".", loci._base_path }

            local ok, res = pcall(function() return p:loadfile(loci._base_path .. "../../proto/game_packets.proto") end)
            if ok and res then
                schema_loaded = true
            else
                local f = io.open(loci._base_path .. "../../proto/game_packets.proto", "r")
                if f then
                    local content = f:read("*a")
                    f:close()
                    if content and p:load(content, "game_packets.proto") then
                        schema_loaded = true
                    end
                end
            end
        end

        -- 2. Fallback: try loading compiled .pb descriptor file
        if not schema_loaded then
            local ok, res = pcall(function() return pb.loadfile(loci._base_path .. "game_packets.pb") end)
            if ok and res then
                schema_loaded = true
            end
        end

        loci._schema_loaded = schema_loaded
        
        if schema_loaded then
            loci._send_intent({ join = { player_name = player_name } })
        else
            print("[Warning] Could not load game_packets schema definition.")
        end
    end
end

function loci.disconnect(reason)
    if loci._udp then
        loci._send_intent({ disconnect = { reason = reason or "client closing" } })
        loci._udp:close()
        loci._udp = nil
    end
end

function loci._send_intent(intent_table)
    if not pb or not loci._schema_loaded or not loci._udp then return end

    loci._sequence_id = loci._sequence_id + 1
    local packet = {
        sequence_id = loci._sequence_id,
        timestamp = math.floor(socket.gettime() * 1000),
        intent = intent_table
    }

    local data = pb.encode("loci2d.GamePacket", packet)
    if data then
        loci._udp:send(data)
    end
end

function loci.send_move(dir_x, dir_y)
    loci._send_intent({ move = { direction = { x_bits = float_to_bits(dir_x), y_bits = float_to_bits(dir_y) } } })
end

function loci.send_action(ability_id, aim_x, aim_y)
    local my_ent = loci.get_my_entity()
    if not my_ent then return end
    
    local dx = aim_x - my_ent.x
    local dy = aim_y - my_ent.y
    local len = math.sqrt(dx * dx + dy * dy)
    
    if len > 0 then
        dx = dx / len
        dy = dy / len
    else
        dx = 0
        dy = 0
    end
    
    loci._send_intent({ action = { ability_id = ability_id, target_direction = { x_bits = float_to_bits(dx), y_bits = float_to_bits(dy) } } })
end

-- TODO: allocates a new table every call; consider caching if GC pressure becomes an issue
function loci.get_entities()
    local list = {}
    for _, e in pairs(loci.entities) do
        table.insert(list, e)
    end
    return list
end

function loci.get_my_entity()
    if loci.my_entity_id then
        return loci.entities[loci.my_entity_id]
    end
    return nil
end

function loci.get_globals()
    return loci.globals
end

function loci.update(dt)
    if not loci._udp then return end

    -- Interpolation
    for _, entity in pairs(loci.entities) do
        if entity.vx ~= 0 or entity.vy ~= 0 then
            entity.x = entity.x + entity.vx * dt
            entity.y = entity.y + entity.vy * dt
        end
    end

    local now = socket.gettime()
    if now - loci._last_heartbeat_time >= 2.0 then
        loci._last_heartbeat_time = now
        loci._send_intent({ ping = {} })
    end

    while true do
        local data, err = loci._udp:receive()
        if not data then
            break
        end

        if loci._schema_loaded then
            local packet = pb.decode("loci2d.ServerPacket", data)
            if packet then
                if packet.world_state then
                    loci._handle_world_state(packet.world_state)
                elseif packet.response then
                    loci._handle_response(packet.response)
                end
            end
        end
    end
end

function loci._handle_world_state(state)
    -- Store globals
    loci.globals = {}
    if state.globals then
        for _, prop in ipairs(state.globals) do
            loci.globals[prop.key] = prop.value
        end
    end
    
    -- We assume the server returns the whole match state, and client just mirrors it
    -- Update my_entity_id if we have a match
    local new_entities = {}
    
    if state.entities then
        for _, raw_ent in ipairs(state.entities) do
            local entity_id = raw_ent.id
            
            -- Detect my_entity
            if loci.my_entity_id == nil and raw_ent.name == loci._player_name then
                loci.my_entity_id = raw_ent.id
            end
            
            local ent = loci.entities[entity_id]
            local is_new = false
            if not ent then
                is_new = true
                ent = {
                    id = entity_id,
                    blueprint = raw_ent.name,
                    properties = {}
                }
            end
            
            -- Update Transform
            if raw_ent.position then
                local nx = bits_to_float(raw_ent.position.x_bits)
                local ny = bits_to_float(raw_ent.position.y_bits)
                ent.x = nx
                ent.y = ny
            else
                ent.x = ent.x or 0
                ent.y = ent.y or 0
            end
            
            -- Update Velocity (for interpolation)
            if raw_ent.velocity then
                ent.vx = bits_to_float(raw_ent.velocity.x_bits)
                ent.vy = bits_to_float(raw_ent.velocity.y_bits)
            else
                ent.vx = 0
                ent.vy = 0
            end
            
            -- Diff properties
            local new_props = {}
            if raw_ent.properties then
                for _, prop in ipairs(raw_ent.properties) do
                    new_props[prop.key] = true
                    local old_val = ent.properties[prop.key]
                    if old_val ~= prop.value then
                        ent.properties[prop.key] = prop.value
                        if not is_new then
                            loci.on_property_changed(ent, prop.key, old_val, prop.value)
                        end
                    end
                end
            end
            -- Check for removed properties (collect first to avoid mutating table during pairs iteration)
            local to_remove = {}
            for k, old_val in pairs(ent.properties) do
                if not new_props[k] then
                    table.insert(to_remove, k)
                    if not is_new then
                        loci.on_property_changed(ent, k, old_val, nil)
                    end
                end
            end
            for _, k in ipairs(to_remove) do
                ent.properties[k] = nil
            end
            
            if is_new then
                loci.entities[entity_id] = ent
                loci.on_entity_spawned(ent)
            end
            
            new_entities[entity_id] = true
        end
    end
    
    -- Despawn entities not in the new state
    for id, ent in pairs(loci.entities) do
        if not new_entities[id] then
            loci.on_entity_despawned(id)
            loci.entities[id] = nil
            if loci.my_entity_id == id then
                loci.my_entity_id = nil
            end
        end
    end
end

function loci._handle_response(resp)
    if resp.status then
        local prefix = "rejected: "
        if string.sub(resp.status, 1, string.len(prefix)) == prefix then
            local reason = string.sub(resp.status, string.len(prefix) + 1)
            loci.on_intent_rejected(reason)
        end
    end
end

return loci
