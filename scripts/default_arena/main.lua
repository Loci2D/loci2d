-- Loci2D Default Arena Server Script

local SPEED = 5.0
local DASH_DISTANCE = 25.0
local DASH_COOLDOWN = 30 -- ticks (~1 segundo a 30-60Hz)
local current_tick = 0
local dash_cooldowns = {}

function on_player_join(entity_id)
    -- The Rust engine already spawned the Player entity with this entity_id
    Loci.Log.info("Player joined with entity ID " .. tostring(entity_id))
    
    -- Set some initial properties for testing UI mapping
    Loci.Commands.set_property(entity_id, "team", "1")
    Loci.Commands.set_property(entity_id, "hp", "100")
end

function on_tick(tick)
    current_tick = tick
end

function on_move_intent(entity_id, dir_x, dir_y)
    Loci.Log.info("Move received: " .. tostring(dir_x) .. ", " .. tostring(dir_y))
    Loci.Commands.set_velocity(entity_id, {x = dir_x * SPEED, y = dir_y * SPEED})
    return true
end

function on_action(entity_id, ability_id, dir_x, dir_y)
    Loci.Log.info("Action received from " .. tostring(entity_id) .. " ability " .. tostring(ability_id))
    
    if ability_id == 3 then
        -- Dash ability (cliente usa ability_id 3)
        local cooldown_end = dash_cooldowns[entity_id] or 0
        if current_tick < cooldown_end then
            return false, "Dash em cooldown"
        end
        
        -- dir_x e dir_y já chegam normalizados do SDK Love2D
        local len_sq = dir_x * dir_x + dir_y * dir_y
        if len_sq > 0.01 then
            local pos = Loci.get_entity_position(entity_id)
            if pos then
                local px, py = pos:x_float(), pos:y_float()
                local new_x = px + dir_x * DASH_DISTANCE
                local new_y = py + dir_y * DASH_DISTANCE
                Loci.Commands.set_position(entity_id, { x = new_x, y = new_y })
                dash_cooldowns[entity_id] = current_tick + DASH_COOLDOWN
                return true
            end
        end
        return false, "Direção inválida para dash"
    end
    
    return false, "Habilidade desconhecida"
end

function on_player_leave(entity_id)
    Loci.Log.info("Player left, entity ID " .. tostring(entity_id))
    Loci.Commands.destroy_entity(entity_id)
end
