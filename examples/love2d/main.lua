-- Love2D Client Example for loci2d using loci_client.lua SDK
-- Pure player-mode client

package.path = package.path .. ";../../sdks/love2d/?.lua"
local loci = require("loci_client")

local last_status = "Connecting to server..."
local rejection_msg = ""
local rejection_timer = 0
local server_ip = "127.0.0.1"
local server_port = 8080

function love.load(args)
    love.window.setTitle("loci2d - Love2D Client SDK Example")
    love.window.setMode(800, 600, { resizable = true })

    -- Connect to the loci2d server
    loci.connect(server_ip, server_port, "Love2DPlayer", "../../sdks/love2d/lib/")
    last_status = "Connected as 'Love2DPlayer'"

    -- Set up callbacks for game events
    loci.on_entity_spawned = function(entity)
        print("New entity spawned:", entity.id)
    end

    loci.on_entity_despawned = function(entity_id)
        print("Entity despawned:", entity_id)
    end

    loci.on_property_changed = function(entity, key, old_val, new_val)
        print("Property changed for " .. tostring(entity.id) .. ": " .. tostring(key) .. " = " .. tostring(new_val))
    end

    loci.on_action_cast = function(entity, ability_id, dir_x, dir_y)
        print("Action cast by " .. tostring(entity.id) .. " ability: " .. tostring(ability_id))
    end

    loci.on_intent_rejected = function(reason)
        print("Server rejected our action:", reason)
        rejection_msg = "Action Failed: " .. reason
        rejection_timer = 3.0 -- Show message for 3 seconds
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

local last_sent_dx, last_sent_dy = 0, 0

function update_movement()
    local dx, dy = get_held_direction()
    if dx ~= last_sent_dx or dy ~= last_sent_dy then
        last_sent_dx = dx
        last_sent_dy = dy
        loci.send_move(dx, dy)
    end
end

function love.keypressed(key)
    if key == "x" or key == "k" then
        -- Explicit stop movement
        last_sent_dx, last_sent_dy = 0, 0
        loci.send_move(0, 0)
    end
end

function love.keyreleased(key)
    -- Input polling is handled in love.update
end

function love.mousepressed(x, y, button)
    -- x, y are Love2D screen coordinates.
    -- Convert to world-space coordinates.
    local my_entity = loci.get_my_entity()
    if not my_entity then return end

    local center_x = love.graphics.getWidth() / 2
    local center_y = love.graphics.getHeight() / 2

    local world_x = my_entity.x + (x - center_x) / 10
    local world_y = my_entity.y + (y - center_y) / 10

    if button == 1 then
        loci.send_action(1, world_x, world_y)
    elseif button == 2 then
        loci.send_action(2, world_x, world_y)
    end
end

function love.update(dt)
    -- Process network packets and update state
    loci.update(dt)

    -- Process robust input polling
    update_movement()

    if rejection_timer > 0 then
        rejection_timer = rejection_timer - dt
        if rejection_timer <= 0 then
            rejection_msg = ""
        end
    end
end

function love.draw()
    -- Background
    love.graphics.clear(0.08, 0.09, 0.13)

    local center_x = love.graphics.getWidth() / 2
    local center_y = love.graphics.getHeight() / 2

    local my_entity = loci.get_my_entity()
    local cam_x = my_entity and my_entity.x or 0
    local cam_y = my_entity and my_entity.y or 0

    -- Draw origin crosshair / grid center relative to camera
    local origin_screen_x = center_x - (cam_x * 10)
    local origin_screen_y = center_y - (cam_y * 10)
    love.graphics.setColor(0.2, 0.25, 0.35, 0.5)
    love.graphics.line(origin_screen_x - 50, origin_screen_y, origin_screen_x + 50, origin_screen_y)
    love.graphics.line(origin_screen_x, origin_screen_y - 50, origin_screen_x, origin_screen_y + 50)
    love.graphics.print("(0, 0)", origin_screen_x + 5, origin_screen_y + 5)

    -- Render all entities from the authoritative world state
    local current_entities = loci.get_entities()
    for _, entity in ipairs(current_entities) do
        local pos_x = center_x + (entity.x - cam_x) * 10
        local pos_y = center_y + (entity.y - cam_y) * 10

        -- Color based on properties if they exist
        local team = entity.properties and entity.properties["team"]
        if team == "1" then
            love.graphics.setColor(0.8, 0.3, 0.3) -- Team 1 Red
        elseif team == "2" then
            love.graphics.setColor(0.3, 0.3, 0.8) -- Team 2 Blue
        else
            love.graphics.setColor(0.3, 0.8, 0.4) -- Default Green
        end

        if loci.my_entity_id == entity.id then
            love.graphics.setColor(0.3, 0.6, 1.0) -- Local Player Blue
        end

        -- Draw Entity avatar
        love.graphics.circle("fill", pos_x, pos_y, 16)
        love.graphics.setColor(1, 1, 1)
        love.graphics.circle("line", pos_x, pos_y, 16)

        -- Draw Entity label
        local label = string.format("%s (id=%d)", entity.blueprint or "Entity", entity.id or 0)
        local font = love.graphics.getFont()
        local text_width = font:getWidth(label)
        love.graphics.setColor(1, 1, 1)
        love.graphics.print(label, pos_x - text_width / 2, pos_y - 32)
        
        -- Draw HP if it exists
        local hp = entity.properties and entity.properties["hp"]
        if hp then
            love.graphics.setColor(1, 0.2, 0.2)
            love.graphics.print("HP: " .. hp, pos_x - 20, pos_y + 20)
        end
    end

    -- HUD / UI Overlay
    love.graphics.setColor(0.12, 0.14, 0.2, 0.85)
    love.graphics.rectangle("fill", 10, 10, 440, 155, 6, 6)
    love.graphics.setColor(0.3, 0.4, 0.6)
    love.graphics.rectangle("line", 10, 10, 440, 155, 6, 6)

    love.graphics.setColor(1, 1, 1)
    love.graphics.print("loci2d - Love2D Client SDK", 20, 20)
    love.graphics.setColor(0.8, 0.8, 0.8)
    love.graphics.print("Controls: WASD / Arrows -> Move | Mouse Click -> Action", 20, 45)
    love.graphics.setColor(0.4, 0.9, 1.0)
    love.graphics.print("Active Entities: " .. tostring(#current_entities), 20, 95)
    love.graphics.setColor(0.9, 0.9, 0.6)
    love.graphics.print("Status: " .. last_status, 20, 120)

    if rejection_msg ~= "" then
        love.graphics.setColor(1, 0.2, 0.2)
        love.graphics.print(rejection_msg, 20, 140)
    end
end

function love.quit()
    loci.disconnect("Client closing")
end
