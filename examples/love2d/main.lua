-- Love2D Client Example for loci2d
-- Demonstrates non-blocking UDP network intent streaming, live WorldState rendering, and Spectator/Replay mode

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

local udp = nil
local sequence_id = 0
local last_status = "Connecting to server..."
local server_ip = "127.0.0.1"
local server_port = 8080
local current_entities = {}
local current_tick = 0
local schema_loaded = false
local last_terminal_print = 0
local last_sent_dir = { x = 0, y = 0 }
local last_packet_time = 0
local is_spectator = false
local last_heartbeat_time = 0

function love.load(args)
    -- Check if launched with --spectator or -s
    if args then
        for _, a in ipairs(args) do
            if a == "--spectator" or a == "-s" or a == "--replay" then
                is_spectator = true
            end
        end
    end

    love.window.setTitle("loci2d - Love2D Client & Replay Viewer")
    love.window.setMode(800, 600, { resizable = true })

    -- Create non-blocking UDP socket
    udp = socket.udp()
    udp:settimeout(0)
    udp:setpeername(server_ip, server_port)

    if pb then
        -- 1. Try dynamic text parsing with protoc.lua
        if protoc then
            local p = protoc.new()
            p.include_dirs = { "../../proto", "proto", "." }

            local ok, res = pcall(function() return p:loadfile("../../proto/game_packets.proto") end)
            if ok and res then
                schema_loaded = true
            else
                -- Fallback: read file text directly via io.open
                local f = io.open("../../proto/game_packets.proto", "r")
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
            local ok, res = pcall(function() return pb.loadfile("game_packets.pb") or pb.loadfile("../../proto/game_packets.pb") end)
            if ok and res then
                schema_loaded = true
            end
        end

        if schema_loaded then
            print("[Protobuf] Successfully loaded game_packets schema.")
            if is_spectator then
                print("[Spectator] Connecting in Spectator / Replay View mode...")
                send_intent({ ping = {} })
                last_status = "Spectator Mode: Watching match stream..."
            else
                send_intent({ join = { player_name = "Love2DPlayer" } })
                last_status = "Player Mode: Joined as 'Love2DPlayer'"
            end
        else
            print("[Warning] Could not load game_packets schema definition.")
            last_status = "Error: game_packets.proto schema not loaded"
        end
    end
end

function love.quit()
    if not is_spectator then
        send_intent({ disconnect = { reason = "closing client" } })
    else
        send_intent({ disconnect = { reason = "spectator closing window" } })
    end
end

function send_intent(intent_table)
    if not pb or not schema_loaded then
        last_status = "Error: Protobuf schema not loaded"
        return
    end

    sequence_id = sequence_id + 1
    local packet = {
        sequence_id = sequence_id,
        timestamp = os.time() * 1000,
        intent = intent_table
    }

    local data = pb.encode("loci2d.GamePacket", packet)
    if data then
        udp:send(data)
        if intent_table.move then
            local dir = intent_table.move.direction or { x_bits = 0, y_bits = 0 }
            local dx = (dir.x_bits or 0) / 65536.0
            local dy = (dir.y_bits or 0) / 65536.0
            print(string.format("[Love2D Client] Sent Move Intent (seq=%d) -> dir=(%.1f, %.1f)", sequence_id, dx, dy))
        elseif intent_table.join then
            print(string.format("[Love2D Client] Sent Join Intent (seq=%d) -> name='%s'", sequence_id, intent_table.join.player_name))
        elseif intent_table.disconnect then
            print(string.format("[Love2D Client] Sent Disconnect Intent (seq=%d) -> reason='%s'", sequence_id, intent_table.disconnect.reason))
        end
    end
end

function get_held_direction()
    local dx, dy = 0, 0
    if love.keyboard.isDown("w") or love.keyboard.isDown("up") then dy = dy - 1 end
    if love.keyboard.isDown("s") or love.keyboard.isDown("down") then dy = dy + 1 end
    if love.keyboard.isDown("a") or love.keyboard.isDown("left") then dx = dx - 1 end
    if love.keyboard.isDown("d") or love.keyboard.isDown("right") then dx = dx + 1 end
    return dx, dy
end

function update_movement()
    if is_spectator then return end
    local dx, dy = get_held_direction()
    if dx ~= last_sent_dir.x or dy ~= last_sent_dir.y then
        last_sent_dir = { x = dx, y = dy }
        send_intent({ move = { direction = { x_bits = math.floor(dx * 65536), y_bits = math.floor(dy * 65536) } } })
    end
end

function love.keypressed(key)
    if key == "tab" or key == "m" then
        -- Toggle Spectator Mode
        is_spectator = not is_spectator
        if is_spectator then
            last_status = "Switched to Spectator Mode (Watching stream)"
            send_intent({ ping = {} })
        else
            last_status = "Switched to Player Mode"
            send_intent({ join = { player_name = "Love2DPlayer" } })
        end
    elseif not is_spectator then
        if key == "w" or key == "s" or key == "a" or key == "d" or key == "up" or key == "down" or key == "left" or key == "right" then
            update_movement()
        elseif key == "x" or key == "k" then
            -- Explicit stop movement
            last_sent_dir = { x = 0, y = 0 }
            send_intent({ move = { direction = { x_bits = 0, y_bits = 0 } } })
        elseif key == "space" then
            send_intent({ action = { ability_id = 1 } })
        elseif key == "p" then
            send_intent({ ping = {} })
        end
    end
end

function love.keyreleased(key)
    if not is_spectator then
        if key == "w" or key == "s" or key == "a" or key == "d" or key == "up" or key == "down" or key == "left" or key == "right" then
            update_movement()
        end
    end
end

function love.update(dt)
    if not udp then return end

    local now = love.timer.getTime()

    -- Send periodic heartbeat ping in spectator mode to maintain stream registration
    if is_spectator and (now - last_heartbeat_time >= 1.0) then
        last_heartbeat_time = now
        send_intent({ ping = {} })
    end

    -- Drain all incoming UDP server packets
    while true do
        local data, msg = udp:receive()
        if not data then break end

        last_packet_time = now

        if pb then
            local server_packet = pb.decode("loci2d.ServerPacket", data)
            if server_packet then
                if server_packet.world_state then
                    current_tick = server_packet.world_state.tick or 0
                    current_entities = server_packet.world_state.entities or {}
                    last_status = string.format("Streaming Tick %d (%d entities active)", current_tick, #current_entities)

                    -- Periodic terminal output for developers (throttled to ~1 second)
                    if now - last_terminal_print >= 1.0 then
                        last_terminal_print = now
                        local entity_strs = {}
                        for _, e in ipairs(current_entities) do
                            local pos = e.position or { x_bits = 0, y_bits = 0 }
                            local px = (pos.x_bits or 0) / 65536.0
                            local py = (pos.y_bits or 0) / 65536.0
                            table.insert(entity_strs, string.format("%s (id=%d) @ (%.1f, %.1f)", e.name or "Entity", e.id or 0, px, py))
                        end
                        print(string.format("[Love2D Client] [Tick %d] %d entities: %s", current_tick, #current_entities, table.concat(entity_strs, " | ")))
                    end

                elseif server_packet.response then
                    last_status = string.format("ACK seq=%d status=%s", server_packet.response.sequence_id, server_packet.response.status)
                end
            end
        end
    end

    -- Connection timeout detection: if server stops sending packets (> 2.0s), clear entities and report status
    if last_packet_time > 0 and (now - last_packet_time > 2.0) then
        if #current_entities > 0 then
            current_entities = {}
        end
        last_status = "Disconnected / Replay Finished (No packets from server)"
    end
end

function love.draw()
    -- Background
    love.graphics.clear(0.08, 0.09, 0.13)

    local center_x = love.graphics.getWidth() / 2
    local center_y = love.graphics.getHeight() / 2

    -- Draw origin crosshair / grid center
    love.graphics.setColor(0.2, 0.25, 0.35, 0.5)
    love.graphics.line(center_x - 50, center_y, center_x + 50, center_y)
    love.graphics.line(center_x, center_y - 50, center_x, center_y + 50)
    love.graphics.print("(0, 0)", center_x + 5, center_y + 5)

    -- Render all synchronized entities from WorldState
    for _, entity in ipairs(current_entities) do
        local pos = entity.position or { x_bits = 0, y_bits = 0 }
        local px = (pos.x_bits or 0) / 65536.0
        local py = (pos.y_bits or 0) / 65536.0
        local pos_x = center_x + px * 10
        local pos_y = center_y + py * 10

        -- Color based on EntityType (0 = Player, 1 = NPC, 2 = Prop)
        if entity.entity_type == 1 or entity.entity_type == "NPC" then
            love.graphics.setColor(0.3, 0.8, 0.4) -- NPC Green
        elseif entity.entity_type == 2 or entity.entity_type == "PROP" then
            love.graphics.setColor(0.7, 0.7, 0.7) -- Prop Gray
        else
            love.graphics.setColor(0.3, 0.6, 1.0) -- Player Blue
        end

        -- Draw Entity avatar
        love.graphics.circle("fill", pos_x, pos_y, 16)
        love.graphics.setColor(1, 1, 1)
        love.graphics.circle("line", pos_x, pos_y, 16)

        -- Draw Entity label
        local label = string.format("%s (id=%d)", entity.name or "Entity", entity.id or 0)
        local font = love.graphics.getFont()
        local text_width = font:getWidth(label)
        love.graphics.setColor(1, 1, 1)
        love.graphics.print(label, pos_x - text_width / 2, pos_y - 32)
    end

    -- HUD / UI Overlay
    love.graphics.setColor(0.12, 0.14, 0.2, 0.85)
    love.graphics.rectangle("fill", 10, 10, 440, 155, 6, 6)
    love.graphics.setColor(0.3, 0.4, 0.6)
    love.graphics.rectangle("line", 10, 10, 440, 155, 6, 6)

    love.graphics.setColor(1, 1, 1)
    local mode_text = is_spectator and "SPECTATOR (Replay Viewer)" or "PLAYER (Live Match)"
    love.graphics.print("loci2d - Love2D Client | Mode: " .. mode_text, 20, 20)

    love.graphics.setColor(0.8, 0.8, 0.8)
    if is_spectator then
        love.graphics.print("Spectator Mode Active: Watching authoritative match stream.", 20, 45)
        love.graphics.print("Press TAB or M to switch to Player Mode.", 20, 65)
    else
        love.graphics.print("Controls: WASD / Arrows -> Move | X -> Stop | Space -> Action", 20, 45)
        love.graphics.print("Press TAB or M to switch to Spectator / Replay Mode.", 20, 65)
    end

    love.graphics.setColor(0.4, 0.9, 1.0)
    love.graphics.print("Tick: " .. tostring(current_tick) .. " | Active Entities: " .. tostring(#current_entities), 20, 95)
    love.graphics.setColor(0.9, 0.9, 0.6)
    love.graphics.print("Status: " .. last_status, 20, 120)
end
