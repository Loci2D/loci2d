-- Loci2D Default Arena Server Script

local SPEED = 5.0

function on_player_join(entity_id)
    -- The Rust engine already spawned the Player entity with this entity_id
    Loci.Log.info("Player joined with entity ID " .. tostring(entity_id))
    
    -- Set some initial properties for testing UI mapping
    Loci.Commands.set_property(entity_id, "team", "1")
    Loci.Commands.set_property(entity_id, "hp", "100")
end

function on_move_intent(entity_id, dir_x, dir_y)
    Loci.Log.info("Move received: " .. tostring(dir_x) .. ", " .. tostring(dir_y))
    -- Artificial rejection test: if moving diagonally, reject it for testing
    if dir_x ~= 0 and dir_y ~= 0 then
        return false, "Diagonal movement is blocked (Testing Rejection!)"
    end

    Loci.Commands.set_velocity(entity_id, {x = dir_x * SPEED, y = dir_y * SPEED})
    return true
end

function on_action(entity_id, ability_id, aim_x, aim_y)
    Loci.Log.info("Action received from " .. tostring(entity_id) .. " ability " .. tostring(ability_id))
    return false, "Abilities are on cooldown!"
end

function on_player_leave(entity_id)
    Loci.Log.info("Player left, entity ID " .. tostring(entity_id))
    Loci.Commands.destroy_entity(entity_id)
end
