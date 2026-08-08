-- Love2D Client Example for loci2d
-- Demonstrates non-blocking UDP network intent streaming with lua-protobuf

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

function love.load()
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
        timestamp = os.time(),
        intent = intent_table
    }

    local data = pb.encode("loci2d.GamePacket", packet)
    if data then
        udp:send(data)
        last_status = "Sent intent seq=" .. sequence_id
    end
end

function love.keypressed(key)
    if key == "w" then
        send_intent({ move = { direction = { x = 0, y = -1 } } })
    elseif key == "s" then
        send_intent({ move = { direction = { x = 0, y = 1 } } })
    elseif key == "a" then
        send_intent({ move = { direction = { x = -1, y = 0 } } })
    elseif key == "d" then
        send_intent({ move = { direction = { x = 1, y = 0 } } })
    elseif key == "space" then
        send_intent({ action = { ability_id = 1 } })
    elseif key == "p" then
        send_intent({ ping = {} })
    end
end

function love.update(dt)
    -- Poll for incoming UDP responses from loci2d server
    local data, msg = udp:receive()
    if data then
        if pb then
            local response = pb.decode("loci2d.ServerResponse", data)
            if response then
                last_status = "Received ACK seq=" .. tostring(response.sequence_id) .. ": " .. tostring(response.status)
            end
        end
    end
end

function love.draw()
    love.graphics.clear(0.1, 0.1, 0.15)
    love.graphics.setColor(1, 1, 1)
    love.graphics.print("loci2d - Love2D Client Example", 20, 20)
    love.graphics.print("Controls:", 20, 50)
    love.graphics.print("  [W, A, S, D] -> Send Movement Intent", 40, 70)
    love.graphics.print("  [Space]      -> Send Action Intent (Ability ID = 1)", 40, 90)
    love.graphics.print("  [P]          -> Send Ping Intent", 40, 110)
    love.graphics.print("Status: " .. last_status, 20, 160)
end
