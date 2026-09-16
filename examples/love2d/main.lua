-- Love2D Client Example for loci2d using loci_client.lua SDK
-- Supports both Player Mode and Spectator Mode (Free Cam & Entity Follow)

package.path = package.path .. ";../../sdks/love2d/?.lua"
local loci = require("loci_client")

local last_status = "Connecting to server..."
local rejection_msg = ""
local rejection_timer = 0
local server_ip = "127.0.0.1"
local server_port = 8080

local visual_fx = {}
local match_banner = ""
local match_banner_timer = 0

local is_spectator_cli = false
local spec_cam_x, spec_cam_y = 0, 0
local following_entity_id = nil
local is_dragging = false
local drag_last_x, drag_last_y = 0, 0

local function get_cam_pos()
    local my_entity = loci.get_my_entity()
    if my_entity then
        return my_entity.x, my_entity.y
    end
    return spec_cam_x, spec_cam_y
end

function love.load(args)
    -- Check CLI args for spectator flag
    local raw_args = args or arg or {}
    for _, v in ipairs(raw_args) do
        if v == "--spectate" or v == "-s" or v == "--replay" or v == "spectate" then
            is_spectator_cli = true
            break
        end
    end

    local client_name = is_spectator_cli and "Spectator" or "Love2DPlayer"
    love.window.setTitle(is_spectator_cli and "loci2d - Spectator Mode" or "loci2d - Love2D Client SDK Example")
    love.window.setMode(800, 600, { resizable = true })

    -- Connect to the loci2d server
    loci.connect(server_ip, server_port, client_name, "../../sdks/love2d/lib/")
    last_status = is_spectator_cli and "Connected as Spectator" or ("Connected as '" .. client_name .. "'")

    -- Set up callbacks for game events
    loci.on_entity_spawned = function(entity)
        print("New entity spawned:", entity.id)
    end

    loci.on_entity_despawned = function(entity_id)
        print("Entity despawned:", entity_id)
        if following_entity_id == entity_id then
            following_entity_id = nil
        end
    end

    loci.on_property_changed = function(entity, key, old_val, new_val)
        print("Property changed for " .. tostring(entity.id) .. ": " .. tostring(key) .. " = " .. tostring(new_val))
    end

    loci.on_action_cast = function(entity, ability_id, dir_x, dir_y)
        local ent_id = entity and entity.id or "?"
        print(string.format("Action cast by %s (ability=%d, dir=[%.2f, %.2f])", tostring(ent_id), ability_id, dir_x, dir_y))
        table.insert(visual_fx, {
            x = entity and entity.x or 0,
            y = entity and entity.y or 0,
            dir_x = dir_x,
            dir_y = dir_y,
            ability_id = ability_id,
            lifetime = 0.35,
            max_lifetime = 0.35,
        })
    end

    loci.on_match_state_changed = function(state, winner)
        print("Match state changed to: " .. tostring(state) .. " winner: " .. tostring(winner))
        match_banner = "MATCH " .. string.upper(state) .. (winner ~= "" and (" (Winner: " .. winner .. ")") or "")
        match_banner_timer = 4.0
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
    elseif key == "r" or key == "space" then
        -- Reset spectator camera
        if not loci.get_my_entity() then
            following_entity_id = nil
            spec_cam_x, spec_cam_y = 0, 0
        end
    end
end

function love.keyreleased(key)
    -- Input polling is handled in love.update
end

function love.mousepressed(x, y, button)
    local my_entity = loci.get_my_entity()
    local cam_x, cam_y = get_cam_pos()
    local center_x = love.graphics.getWidth() / 2
    local center_y = love.graphics.getHeight() / 2

    -- Avoid clicking through the HUD overlay
    if x >= 10 and x <= 480 and y >= 10 and y <= 175 then
        return
    end

    if my_entity then
        local world_x = my_entity.x + (x - center_x) / 10
        local world_y = my_entity.y + (y - center_y) / 10

        if button == 1 then
            loci.send_action(1, world_x, world_y)
        elseif button == 2 then
            loci.send_action(2, world_x, world_y)
        end
    else
        -- Spectator interactions: Click to follow entity, or Drag to pan camera
        local clicked_world_x = cam_x + (x - center_x) / 10
        local clicked_world_y = cam_y + (y - center_y) / 10

        if button == 1 then
            local clicked_entity = nil
            for _, ent in ipairs(loci.get_entities()) do
                local d2 = (ent.x - clicked_world_x)^2 + (ent.y - clicked_world_y)^2
                if d2 < 9 then -- within ~3 units radius
                    clicked_entity = ent
                    break
                end
            end

            if clicked_entity then
                following_entity_id = clicked_entity.id
            else
                following_entity_id = nil
                is_dragging = true
                drag_last_x, drag_last_y = x, y
            end
        elseif button == 2 or button == 3 then
            following_entity_id = nil
            is_dragging = true
            drag_last_x, drag_last_y = x, y
        end
    end
end

function love.mousereleased(x, y, button)
    if button == 1 or button == 2 or button == 3 then
        is_dragging = false
    end
end

function love.mousemoved(x, y, dx, dy)
    if is_dragging then
        following_entity_id = nil
        -- 10 pixels = 1 world unit
        spec_cam_x = spec_cam_x - dx / 10
        spec_cam_y = spec_cam_y - dy / 10
    end
end

function love.update(dt)
    -- Process network packets and update state
    loci.update(dt)

    local my_entity = loci.get_my_entity()
    local is_spectating = is_spectator_cli or (my_entity == nil)

    if my_entity then
        -- Process robust input polling for player entity
        update_movement()
    else
        -- Spectator mode free camera & follow logic
        local kdx, kdy = get_held_direction()
        if kdx ~= 0 or kdy ~= 0 then
            following_entity_id = nil
            local cam_speed = 35 -- units per second
            spec_cam_x = spec_cam_x + kdx * cam_speed * dt
            spec_cam_y = spec_cam_y + kdy * cam_speed * dt
        end

        if following_entity_id then
            local target_ent = loci.entities[following_entity_id]
            if target_ent then
                spec_cam_x = target_ent.x
                spec_cam_y = target_ent.y
            else
                following_entity_id = nil
            end
        end
    end

    for i = #visual_fx, 1, -1 do
        local fx = visual_fx[i]
        fx.lifetime = fx.lifetime - dt
        if fx.lifetime <= 0 then
            table.remove(visual_fx, i)
        end
    end

    if match_banner_timer > 0 then
        match_banner_timer = match_banner_timer - dt
        if match_banner_timer <= 0 then
            match_banner = ""
        end
    end

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
    local is_spectating = is_spectator_cli or (my_entity == nil)
    local cam_x, cam_y = get_cam_pos()

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

        -- Direct typed property access (Phase 6.5.3-1)
        local team = entity.team
        if team == 1 or team == "1" then
            love.graphics.setColor(0.8, 0.3, 0.3) -- Team 1 Red
        elseif team == 2 or team == "2" then
            love.graphics.setColor(0.3, 0.3, 0.8) -- Team 2 Blue
        else
            love.graphics.setColor(0.3, 0.8, 0.4) -- Default Green
        end

        if entity:is_local_player() then
            love.graphics.setColor(0.3, 0.6, 1.0) -- Local Player Blue
        end

        -- Draw Entity avatar
        love.graphics.circle("fill", pos_x, pos_y, 16)
        love.graphics.setColor(1, 1, 1)
        love.graphics.circle("line", pos_x, pos_y, 16)

        -- Highlight followed target in spectator mode
        if is_spectating and following_entity_id == entity.id then
            love.graphics.setColor(1, 0.85, 0.2, 0.85)
            love.graphics.circle("line", pos_x, pos_y, 22)
            love.graphics.print("[Target]", pos_x - 22, pos_y + 20)
        end

        -- Draw Entity label
        local label = string.format("%s (id=%d)", entity.blueprint or "Entity", entity.id or 0)
        local font = love.graphics.getFont()
        local text_width = font:getWidth(label)
        love.graphics.setColor(1, 1, 1)
        love.graphics.print(label, pos_x - text_width / 2, pos_y - 32)
        
        -- Draw HP if it exists (using direct typed property access)
        local hp = entity.hp
        if hp and (not is_spectating or following_entity_id ~= entity.id) then
            love.graphics.setColor(1, 0.2, 0.2)
            love.graphics.print("HP: " .. tostring(hp), pos_x - 20, pos_y + 20)
        end
    end

    -- Render transient action visual effects (Phase 6.5.3-1)
    for _, fx in ipairs(visual_fx) do
        local alpha = math.max(0, fx.lifetime / fx.max_lifetime)
        local start_x = center_x + (fx.x - cam_x) * 10
        local start_y = center_y + (fx.y - cam_y) * 10
        local end_x = start_x + (fx.dir_x * 45)
        local end_y = start_y + (fx.dir_y * 45)

        if fx.ability_id == 1 then
            love.graphics.setColor(1, 0.85, 0.2, alpha) -- Ability 1: Yellow Beam
            love.graphics.setLineWidth(3)
        else
            love.graphics.setColor(0.85, 0.2, 1, alpha) -- Ability 2: Purple Beam
            love.graphics.setLineWidth(5)
        end
        love.graphics.line(start_x, start_y, end_x, end_y)
        love.graphics.circle("fill", end_x, end_y, 4 * alpha)
        love.graphics.setLineWidth(1)
    end

    -- Match State Notification Banner
    if match_banner ~= "" then
        local mb_w = 320
        local mb_h = 32
        local mb_x = center_x - mb_w / 2
        local mb_y = is_spectating and 48 or 14
        love.graphics.setColor(0.1, 0.15, 0.25, 0.92)
        love.graphics.rectangle("fill", mb_x, mb_y, mb_w, mb_h, 8, 8)
        love.graphics.setColor(0.4, 0.8, 1.0)
        love.graphics.rectangle("line", mb_x, mb_y, mb_w, mb_h, 8, 8)
        love.graphics.setColor(1, 1, 1)
        local font = love.graphics.getFont()
        local tw = font:getWidth(match_banner)
        love.graphics.print(match_banner, center_x - tw / 2, mb_y + 8)
    end

    -- Top Center Spectator Pill / Banner (Unmistakable visual indicator)
    if is_spectating then
        local banner_w = 200
        local banner_h = 28
        local bx = center_x - banner_w / 2
        local by = 12
        love.graphics.setColor(0.08, 0.10, 0.16, 0.92)
        love.graphics.rectangle("fill", bx, by, banner_w, banner_h, 14, 14)
        love.graphics.setColor(0.95, 0.45, 0.15, 0.9)
        love.graphics.setLineWidth(1.5)
        love.graphics.rectangle("line", bx, by, banner_w, banner_h, 14, 14)
        love.graphics.setLineWidth(1)

        -- Red live stream indicator dot
        love.graphics.setColor(1.0, 0.25, 0.25)
        love.graphics.circle("fill", bx + 22, by + 14, 4.5)

        love.graphics.setColor(1, 1, 1)
        love.graphics.print("SPECTATOR MODE", bx + 36, by + 7)
    end

    -- HUD / UI Overlay (Top Left Panel)
    local hud_w, hud_h = 470, 155
    love.graphics.setColor(0.10, 0.12, 0.18, 0.88)
    love.graphics.rectangle("fill", 10, 10, hud_w, hud_h, 8, 8)
    love.graphics.setColor(0.25, 0.35, 0.5)
    love.graphics.rectangle("line", 10, 10, hud_w, hud_h, 8, 8)

    love.graphics.setColor(1, 1, 1)
    love.graphics.print("loci2d - Client", 20, 20)

    if is_spectating then
        -- Spectator tag badge
        love.graphics.setColor(0.85, 0.4, 0.15, 0.9)
        love.graphics.rectangle("fill", 130, 18, 140, 20, 4, 4)
        love.graphics.setColor(1, 1, 1)
        love.graphics.print("[SPECTATOR MODE]", 138, 21)

        -- Controls for Spectator
        love.graphics.setColor(0.95, 0.85, 0.5)
        love.graphics.print("Controls: WASD / Drag -> Pan Cam | Click -> Follow | Space -> Reset", 20, 48)

        -- Camera status line
        love.graphics.setColor(0.7, 0.85, 1.0)
        if following_entity_id then
            love.graphics.print("Camera: Following Entity #" .. tostring(following_entity_id), 20, 72)
        else
            love.graphics.print(string.format("Camera: Free Cam (x: %.1f, y: %.1f)", cam_x, cam_y), 20, 72)
        end
    else
        -- Player tag badge
        love.graphics.setColor(0.2, 0.65, 0.35, 0.9)
        love.graphics.rectangle("fill", 130, 18, 125, 20, 4, 4)
        love.graphics.setColor(1, 1, 1)
        love.graphics.print("[PLAYER MODE]", 142, 21)

        -- Controls for Player
        love.graphics.setColor(0.85, 0.85, 0.85)
        love.graphics.print("Controls: WASD / Arrows -> Move | Mouse Click -> Action", 20, 48)

        -- Player entity info
        love.graphics.setColor(0.4, 0.9, 1.0)
        love.graphics.print(string.format("Player Entity: %s (id=%d)", my_entity.blueprint or loci._player_name, my_entity.id or 0), 20, 72)
    end

    love.graphics.setColor(0.4, 0.9, 1.0)
    love.graphics.print("Active Entities: " .. tostring(#current_entities), 20, 96)
    love.graphics.setColor(0.9, 0.9, 0.6)
    love.graphics.print("Status: " .. last_status, 20, 120)

    if rejection_msg ~= "" then
        love.graphics.setColor(1, 0.2, 0.2)
        love.graphics.print(rejection_msg, 20, 138)
    end
end

function love.quit()
    loci.disconnect("Client closing")
end
