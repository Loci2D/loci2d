-- Love2D Client Example for loci2d
-- Demonstrates non-blocking UDP network intent streaming and live WorldState rendering

local socket = require("socket")
local pb = nil

-- Attempt to load lua-protobuf library
local ok, res = pcall(require, "pb")
if ok then
    pb = res
else
    print("[Warning] 'lua-protobuf' module not found.")
    print("Install lua-protobuf (e.g. via luarocks install lua-protobuf) to run full binary Protobuf encoding.")
end

local udp = nil
local sequence_id = 0
local last_status = "Press WASD/Space to send intents to server"
local server_ip = "127.0.0.1"
local server_port = 8080
local current_entities = {}
local current_tick = 0

function love.load()
    love.window.setTitle("loci2d - Love2D Client (Phase 3)")
    love.window.setMode(800, 600, { resizable = true })

    -- Create non-blocking UDP socket
    udp = socket.udp()
    udp:settimeout(0)
    udp:setpeername(server_ip, server_port)

    if pb then
        -- Load proto file dynamically
        local loaded = pb.loadfile("../../proto/game_packets.proto")
        if not loaded then
            print("[Warning] Could not load ../../proto/game_packets.proto")
        else
            -- Automatically join instance on startup
            send_intent({ join = { player_name = "Love2DPlayer" } })
        end
    end
end

function love.quit()
    send_intent({ disconnect = { reason = "closing client" } })
end

function send_intent(intent_table)
    if not pb then
        last_status = "Error: lua-protobuf not installed"
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
        last_status = "Sent intent seq=" .. sequence_id
    end
end

function love.keypressed(key)
    if key == "w" or key == "up" then
        send_intent({ move = { direction = { x = 0, y = -1 } } })
    elseif key == "s" or key == "down" then
        send_intent({ move = { direction = { x = 0, y = 1 } } })
    elseif key == "a" or key == "left" then
        send_intent({ move = { direction = { x = -1, y = 0 } } })
    elseif key == "d" or key == "right" then
        send_intent({ move = { direction = { x = 1, y = 0 } } })
    elseif key == "space" then
        send_intent({ action = { ability_id = 1 } })
    elseif key == "p" then
        send_intent({ ping = {} })
    end
end

function love.update(dt)
    if not udp then return end

    -- Drain all incoming UDP server packets
    while true do
        local data, msg = udp:receive()
        if not data then break end

        if pb then
            local server_packet = pb.decode("loci2d.ServerPacket", data)
            if server_packet then
                if server_packet.world_state then
                    current_tick = server_packet.world_state.tick or 0
                    current_entities = server_packet.world_state.entities or {}
                    last_status = string.format("WorldState Tick %d (%d entities)", current_tick, #current_entities)
                elseif server_packet.response then
                    last_status = string.format("ACK seq=%d status=%s", server_packet.response.sequence_id, server_packet.response.status)
                end
            end
        end
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
        local pos_x = center_x + (entity.position and entity.position.x or 0) * 10
        local pos_y = center_y + (entity.position and entity.position.y or 0) * 10

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
    love.graphics.rectangle("fill", 10, 10, 360, 140, 6, 6)
    love.graphics.setColor(0.3, 0.4, 0.6)
    love.graphics.rectangle("line", 10, 10, 360, 140, 6, 6)

    love.graphics.setColor(1, 1, 1)
    love.graphics.print("loci2d - Love2D Client (Phase 3)", 20, 20)
    love.graphics.setColor(0.8, 0.8, 0.8)
    love.graphics.print("Controls: WASD / Arrow Keys -> Move | Space -> Action", 20, 45)
    love.graphics.print("          P -> Ping", 20, 65)

    love.graphics.setColor(0.4, 0.9, 1.0)
    love.graphics.print("Tick: " .. tostring(current_tick) .. " | Active Entities: " .. tostring(#current_entities), 20, 95)
    love.graphics.setColor(0.9, 0.9, 0.6)
    love.graphics.print("Status: " .. last_status, 20, 115)
end
