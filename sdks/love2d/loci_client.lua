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

local function bits_to_float(bits)
    if not bits then return 0.0 end
    return bits / 65536.0
end

local function float_to_bits(f)
    if not f then return 0 end
    return math.floor(f * 65536)
end

local function cast_property_value(val)
    if type(val) ~= "string" then return val end
    if val == "true" or val == "True" then return true end
    if val == "false" or val == "False" then return false end
    local num = tonumber(val)
    if num ~= nil then return num end
    return val
end

-- ============================================================================
-- High-Level Entity Abstraction (Phase 6.5.3-1)
-- ============================================================================
local Entity = {}
Entity.__index = function(t, k)
    local method = Entity[k]
    if method ~= nil then return method end
    local props = rawget(t, "properties")
    if props ~= nil then
        return props[k]
    end
    return nil
end

function Entity:is_local_player()
    return loci and self.id == loci.my_entity_id
end

function Entity:distance_to(other_or_pos)
    if not other_or_pos then return 0 end
    local ox = other_or_pos.x or 0
    local oy = other_or_pos.y or 0
    local dx = self.x - ox
    local dy = self.y - oy
    return math.sqrt(dx * dx + dy * dy)
end

function Entity:get(prop_name, default_value)
    local props = rawget(self, "properties")
    if props and props[prop_name] ~= nil then
        return props[prop_name]
    end
    return default_value
end

local function new_entity(id, blueprint)
    local ent = {
        id = id,
        blueprint = blueprint,
        x = 0,
        y = 0,
        vx = 0,
        vy = 0,
        properties = {}
    }
    setmetatable(ent, Entity)
    return ent
end

-- ============================================================================
-- Loci Client Module Facade
-- ============================================================================
local loci = {
    -- State
    SCHEMA_VERSION = 1,
    entities = {},
    globals = {},
    my_entity_id = nil,
    match_state = "running",
    match_winner = "",

    -- Callbacks
    on_entity_spawned = function(entity) end,
    on_entity_despawned = function(entity_id) end,
    on_property_changed = function(entity, key, old_val, new_val) end,
    on_match_state_changed = function(state, winner) end,
    on_action_cast = function(entity, ability_id, dir_x, dir_y) end,
    on_intent_rejected = function(reason) end,
    
    -- Internal
    _udp = nil,
    _schema_loaded = false,
    _sequence_id = 0,
    _last_heartbeat_time = 0,
    _player_name = nil,
    _base_path = "lib/",
    _entities_list_cache = nil,
}

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
            p.include_dirs = { loci._base_path, loci._base_path .. "../../proto", "proto", "." }

            local ok, res = pcall(function() return p:loadfile(loci._base_path .. "game_packets.proto") end)
            if ok and res then
                schema_loaded = true
            else
                local f = io.open(loci._base_path .. "game_packets.proto", "r")
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
            loci._send_intent({ join = { player_name = player_name, schema_version = loci.SCHEMA_VERSION } })
            return true
        else
            print("[Error] Could not load game_packets schema definition. Aborting network.")
            loci._udp:close()
            loci._udp = nil
            return false
        end
    end
    
    return false
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

-- Cached to avoid GC pressure
function loci.get_entities()
    if not loci._entities_list_cache then
        local list = {}
        for _, e in pairs(loci.entities) do
            table.insert(list, e)
        end
        loci._entities_list_cache = list
    end
    return loci._entities_list_cache
end

function loci.get_entities_by_blueprint(blueprint_name)
    local list = {}
    for _, e in pairs(loci.entities) do
        if e.blueprint == blueprint_name then
            table.insert(list, e)
        end
    end
    return list
end

function loci.get_entities_in_radius(center_x, center_y, radius)
    local list = {}
    local r2 = radius * radius
    for _, e in pairs(loci.entities) do
        local dx = e.x - center_x
        local dy = e.y - center_y
        if (dx * dx + dy * dy) <= r2 then
            table.insert(list, e)
        end
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
        if entity.server_x and entity.server_y then
            -- Predict theoretical server position
            entity.server_x = entity.server_x + entity.vx * dt
            entity.server_y = entity.server_y + entity.vy * dt
            
            local dx = entity.server_x - entity.x
            local dy = entity.server_y - entity.y
            local dist2 = dx * dx + dy * dy
            
            if dist2 > 4.0 then
                -- Strict snap (Rubberbanding > 2.0 units)
                entity.x = entity.server_x
                entity.y = entity.server_y
            elseif dist2 > 0.001 then
                -- Soft lerp
                entity.x = entity.x + dx * 10.0 * dt
                entity.y = entity.y + dy * 10.0 * dt
            else
                -- Just move with velocity
                entity.x = entity.x + entity.vx * dt
                entity.y = entity.y + entity.vy * dt
            end
        else
            -- No server pos yet, just use vx/vy
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
        local data, err = loci._udp:receive(65536)
        if not data then
            break
        end

        if loci._schema_loaded then
            local ok, packet = pcall(pb.decode, "loci2d.ServerPacket", data)
            if ok and packet then
                if packet.world_state then
                    loci._handle_world_state(packet.world_state)
                elseif packet.response then
                    loci._handle_response(packet.response)
                end
            elseif not ok then
                print("[Error] pb.decode failed: " .. tostring(packet))
            end
        end
    end
end

function loci._handle_world_state(state)
    -- Store globals with auto-type casting
    loci.globals = {}
    if state.globals then
        for _, prop in ipairs(state.globals) do
            loci.globals[prop.key] = cast_property_value(prop.value)
        end
    end

    -- Match state synchronization
    if state.match_state ~= nil then
        local state_map = { [0] = "running", [1] = "paused", [2] = "ended" }
        local new_state = state_map[state.match_state] or "running"
        local new_winner = state.match_winner or ""
        if loci.match_state ~= new_state or (new_state == "ended" and loci.match_winner ~= new_winner) then
            loci.match_state = new_state
            loci.match_winner = new_winner
            loci.on_match_state_changed(new_state, new_winner)
        end
    end
    
    -- Ingest and diff entities
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
                ent = new_entity(entity_id, raw_ent.name)
            end
            
            -- Update Transform
            if raw_ent.position then
                local nx = bits_to_float(raw_ent.position.x_bits)
                local ny = bits_to_float(raw_ent.position.y_bits)
                ent.server_x = nx
                ent.server_y = ny
                if is_new then
                    ent.x = nx
                    ent.y = ny
                end
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
            
            -- Diff properties with auto-casting
            local new_props = {}
            if raw_ent.properties then
                for _, prop in ipairs(raw_ent.properties) do
                    new_props[prop.key] = true
                    local casted_val = cast_property_value(prop.value)
                    local old_val = ent.properties[prop.key]
                    if old_val ~= casted_val then
                        ent.properties[prop.key] = casted_val
                        if not is_new then
                            loci.on_property_changed(ent, prop.key, old_val, casted_val)
                        end
                    end
                end
            end
            -- Check for removed properties
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
                loci._entities_list_cache = nil
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
            loci._entities_list_cache = nil
            if loci.my_entity_id == id then
                loci.my_entity_id = nil
            end
        end
    end

    -- Process transient action broadcasts
    if state.actions then
        for _, act in ipairs(state.actions) do
            local dir_x, dir_y = 0.0, 0.0
            if act.target_direction then
                dir_x = bits_to_float(act.target_direction.x_bits)
                dir_y = bits_to_float(act.target_direction.y_bits)
            end
            local ent = loci.entities[act.entity_id]
            if ent then
                loci.on_action_cast(ent, act.ability_id, dir_x, dir_y)
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
