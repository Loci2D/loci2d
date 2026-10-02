-- Love2D Client Example for loci2d using loci_client.lua SDK
-- Supports both Player Mode and Spectator Mode (Free Cam & Entity Follow)

package.path = package.path .. ";../../sdks/love2d/?.lua;../../sdks/love2d/lib/?.lua;sdks/love2d/?.lua;sdks/love2d/lib/?.lua;./?.lua;./lib/?.lua"
package.cpath = package.cpath .. ";../../sdks/love2d/lib/?.so;../../sdks/love2d/?.so;sdks/love2d/lib/?.so;sdks/love2d/lib/?.so;./?.so;./lib/?.so"
local loci = require("loci_client")
local anim8 = require("anim8")

local last_status = "Connecting to server..."
local rejection_msg = ""
local rejection_timer = 0
local server_ip = "192.168.0.2"
local server_port = 8080

local visual_fx = {}
local match_banner = ""
local match_banner_timer = 0

local is_spectator_cli = false
local spec_cam_x, spec_cam_y = 0, 0
local following_entity_id = nil
local is_dragging = false
local drag_last_x, drag_last_y = 0, 0

-- Sistema de Dash - detecção de double-tap
local last_key_time = {}
local DASH_DOUBLE_TAP_TIME = 0.3  -- segundos entre presses para detectar double-tap
local DASH_COOLDOWN = 1.0  -- segundos entre dashes
local last_dash_time = 0

-- Static obstacles for 100x100 map (manual definition since server doesn't send them)
local static_obstacles = {
    -- Walls (100x100 map from -50 to 50, with 2 unit thickness)
    { type = "aabb", min = { x = -52, y = -50 }, max = { x = -50, y = 50 } },  -- Left wall
    { type = "aabb", min = { x = 50, y = -50 }, max = { x = 52, y = 50 } },    -- Right wall
    { type = "aabb", min = { x = -50, y = -52 }, max = { x = 50, y = -50 } },  -- Top wall
    { type = "aabb", min = { x = -50, y = 50 }, max = { x = 50, y = 52 } },    -- Bottom wall
    -- Central circle obstacle
    { type = "circle", center = { x = 0, y = 0 }, radius = 5.0 },
    -- Corner obstacles
    { type = "aabb", min = { x = -30, y = -30 }, max = { x = -25, y = -25 } },
    { type = "aabb", min = { x = 25, y = -30 }, max = { x = 30, y = -25 } },
    { type = "aabb", min = { x = -30, y = 25 }, max = { x = -25, y = 30 } },
    { type = "aabb", min = { x = 25, y = 25 }, max = { x = 30, y = 30 } },
    -- Middle obstacles
    { type = "aabb", min = { x = -10, y = -20 }, max = { x = -5, y = -15 } },
    { type = "aabb", min = { x = 5, y = -20 }, max = { x = 10, y = -15 } },
    { type = "aabb", min = { x = -10, y = 15 }, max = { x = -5, y = 20 } },
    { type = "aabb", min = { x = 5, y = 15 }, max = { x = 10, y = 20 } },
}

-- ===== Sprite / Animação =====
local SPRITE_FRAME_W, SPRITE_FRAME_H = 48, 48
local SPRITE_SCALE = 6
local knight_img
local knight_grid
local anim_templates

-- Fireball spritesheet
local FIREBALL_FRAME_W, FIREBALL_FRAME_H = 13, 9
local FIREBALL_FRAME_COUNT = 7
local FIREBALL_FRAME_DURATION = 0.05
local FIREBALL_SCALE = 3
local fireball_img
local fireball_grid
local fireball_anim_template

-- Estado de animação por entidade
local entity_anims = {}
local MOVE_EPSILON = 0.05

local function get_entity_kind(entity)
    if entity.blueprint == "fireball" then
        return "fireball"
    end
    if entity.properties and entity.properties["kind"] == "fireball" then
        return "fireball"
    end
    return "player"
end

local function make_entity_anim(kind)
    if kind == "fireball" then
        return {
            kind = "fireball",
            anim = fireball_anim_template:clone(),
            rotation = 0,
            prev_x = nil,
            prev_y = nil,
        }
    end
    return {
        kind = "player",
        anim = anim_templates.idle:clone(),
        state = "idle",
        facing = 1,
        prev_x = nil,
        prev_y = nil,
    }
end

local function update_entity_anim(id, entity, dt)
    local kind = get_entity_kind(entity)
    local state = entity_anims[id]
    if not state or state.kind ~= kind then
        state = make_entity_anim(kind)
        state.prev_x, state.prev_y = entity.x, entity.y
        entity_anims[id] = state
    end

    local dx = entity.x - (state.prev_x or entity.x)
    local dy = entity.y - (state.prev_y or entity.y)
    state.prev_x, state.prev_y = entity.x, entity.y

    if kind == "fireball" then
        if (dx * dx + dy * dy) > (MOVE_EPSILON * MOVE_EPSILON) then
            state.rotation = math.atan2(dy, dx) - math.pi
        end
        state.anim:update(dt)
        return
    end

    if dx > MOVE_EPSILON then
        state.facing = 1
    elseif dx < -MOVE_EPSILON then
        state.facing = -1
    end

    local moved = (dx * dx + dy * dy) > (MOVE_EPSILON * MOVE_EPSILON)
    local target_state = moved and "walk" or "idle"

    if target_state ~= state.state then
        state.state = target_state
        state.anim = anim_templates[target_state]:clone()
    end

    state.anim:update(dt)
end



-- DASH

local DASH_DISTANCE = 25.0
local DASH_COOLDOWN = 30 -- ticks (~1 segundo a 30-60Hz)
local current_tick = 0
local dash_cooldowns = {}

function on_tick(tick)
    current_tick = tick
end

function on_action(entity_id, ability_id, dir_x, dir_y)
    if ability_id == 2 then
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


-- ============================================================
-- Colisão de skills (projéteis somem ao bater em qualquer coisa)
-- ============================================================
local PROJECTILE_RADIUS = 0.8 -- raio do fireball (8px / 10)
local ENTITY_RADIUS = 1.6     -- raio dos players (16px / 10)
local BEAM_LEN = 4.5          -- tamanho do flash de cast em unidades (45px / 10)

local projectile_state = {}   -- [id] = { px, py, owner, dead }

local function is_projectile(ent)
    return ent.blueprint == "fireball" or (ent.properties and ent.properties.kind == "fireball")
end

-- Segmento vs AABB (slab). Retorna t de entrada (0..1) ou nil
local function seg_vs_aabb(x0, y0, x1, y1, minx, miny, maxx, maxy)
    local dx, dy = x1 - x0, y1 - y0
    local t0, t1 = 0, 1

    if math.abs(dx) < 1e-9 then
        if x0 < minx or x0 > maxx then return nil end
    else
        local ta, tb = (minx - x0) / dx, (maxx - x0) / dx
        if ta > tb then ta, tb = tb, ta end
        t0, t1 = math.max(t0, ta), math.min(t1, tb)
        if t0 > t1 then return nil end
    end

    if math.abs(dy) < 1e-9 then
        if y0 < miny or y0 > maxy then return nil end
    else
        local ta, tb = (miny - y0) / dy, (maxy - y0) / dy
        if ta > tb then ta, tb = tb, ta end
        t0, t1 = math.max(t0, ta), math.min(t1, tb)
        if t0 > t1 then return nil end
    end

    return t0
end

-- Segmento vs círculo. Retorna t de entrada (0..1) ou nil
local function seg_vs_circle(x0, y0, x1, y1, cx, cy, r)
    local dx, dy = x1 - x0, y1 - y0
    local fx, fy = x0 - cx, y0 - cy
    local c = fx * fx + fy * fy - r * r
    if c <= 0 then return 0 end
    local a = dx * dx + dy * dy
    if a < 1e-12 then return nil end
    local b = fx * dx + fy * dy
    local disc = b * b - a * c
    if disc < 0 then return nil end
    local t = (-b - math.sqrt(disc)) / a
    if t >= 0 and t <= 1 then return t end
    return nil
end

-- Primeiro impacto ao longo do segmento (paredes, obstáculos e entidades).
-- Retorna t (0..1) ou nil. ignore_id = dono da skill (não colide com ele).
local function first_hit_t(x0, y0, x1, y1, radius, ignore_id)
    local best = nil

    for _, obs in ipairs(static_obstacles) do
        local t
        if obs.type == "aabb" then
            t = seg_vs_aabb(x0, y0, x1, y1,
                obs.min.x - radius, obs.min.y - radius,
                obs.max.x + radius, obs.max.y + radius)
        else
            t = seg_vs_circle(x0, y0, x1, y1, obs.center.x, obs.center.y, obs.radius + radius)
        end
        if t and (not best or t < best) then best = t end
    end

    for _, e in ipairs(loci.get_entities()) do
        if e.id ~= ignore_id and not is_projectile(e) then
            local t = seg_vs_circle(x0, y0, x1, y1, e.x, e.y, ENTITY_RADIUS + radius)
            if t and (not best or t < best) then best = t end
        end
    end

    return best
end

-- Dono da skill = entidade não-projétil mais próxima no spawn
local function find_owner(proj)
    local best_id, best_d2 = nil, 9 -- até 3 unidades
    for _, e in ipairs(loci.get_entities()) do
        if not is_projectile(e) then
            local d2 = (e.x - proj.x) ^ 2 + (e.y - proj.y) ^ 2
            if d2 < best_d2 then best_id, best_d2 = e.id, d2 end
        end
    end
    return best_id
end

local function is_projectile_dead(ent)
    local st = projectile_state[ent.id]
    return st ~= nil and st.dead
end

local TRAIL_LIFE = 0.35
local impact_fx = {} -- explosões de impacto (coordenadas de mundo)

local function spawn_impact(x, y)
    local sparks = {}
    for i = 1, 12 do
        local a = math.random() * math.pi * 2
        sparks[i] = { dx = math.cos(a), dy = math.sin(a), speed = 25 + math.random() * 45 }
    end
    table.insert(impact_fx, { x = x, y = y, age = 0, life = 0.45, sparks = sparks })
end

local function update_projectiles(dt)
    for _, e in ipairs(loci.get_entities()) do
        if is_projectile(e) then
            local st = projectile_state[e.id]
            if not st then
                st = { px = e.x, py = e.y, owner = find_owner(e), dead = false, trail = {}, dx = 0, dy = 0 }
                projectile_state[e.id] = st
            end

            -- Envelhece a trilha (continua sumindo mesmo depois do impacto)
            local trail = st.trail
            for i = #trail, 1, -1 do
                trail[i].age = trail[i].age + dt
                if trail[i].age > TRAIL_LIFE then table.remove(trail, i) end
            end

            if not st.dead then
                -- Teste "swept": do frame anterior até agora (não atravessa parede fina)
                local t = first_hit_t(st.px, st.py, e.x, e.y, PROJECTILE_RADIUS, st.owner)
                if t then
                    st.dead = true
                    spawn_impact(st.px + (e.x - st.px) * t, st.py + (e.y - st.py) * t)
                else
                    -- Verificar colisão com outras fireballs
                    for _, other in ipairs(loci.get_entities()) do
                        if is_projectile(other) and other.id ~= e.id then
                            local dx = other.x - e.x
                            local dy = other.y - e.y
                            local dist = math.sqrt(dx * dx + dy * dy)
                            if dist < PROJECTILE_RADIUS * 2 then
                                st.dead = true
                                spawn_impact(e.x, e.y)
                                break
                            end
                        end
                    end
                    
                    if not st.dead then
                        local mx, my = e.x - st.px, e.y - st.py
                        local len = math.sqrt(mx * mx + my * my)
                        if len > 0.01 then
                            st.dx, st.dy = mx / len, my / len
                            table.insert(trail, {
                                x = e.x, y = e.y, age = 0,
                                ox = (math.random() - 0.5) * 2, oy = (math.random() - 0.5) * 2,
                            })
                        end
                        st.px, st.py = e.x, e.y
                    end
                end
            end
        end
    end

    for i = #impact_fx, 1, -1 do
        impact_fx[i].age = impact_fx[i].age + dt
        if impact_fx[i].age >= impact_fx[i].life then table.remove(impact_fx, i) end
    end
end

-- Desenha trilha + cabeça de fogo do projétil (blend aditivo = brilho)
local function draw_fireball(ent, pos_x, pos_y, cam_x, cam_y, cx, cy)
    local st = projectile_state[ent.id]
    local t = love.timer.getTime()
    love.graphics.setBlendMode("add")

    if st then
        for _, p in ipairs(st.trail) do
            local k = 1 - p.age / TRAIL_LIFE
            local sx = cx + (p.x - cam_x) * 10
            local sy = cy + (p.y - cam_y) * 10
            love.graphics.setColor(1.0, 0.2 + 0.45 * k, 0.03, 0.30 * k)
            love.graphics.circle("fill", sx, sy, 3 + 9 * k)
            -- fagulha que se afasta da trilha
            love.graphics.setColor(1.0, 0.9, 0.4, 0.9 * k)
            love.graphics.circle("fill", sx + p.ox * p.age * 30, sy + p.oy * p.age * 30, 0.5 + 1.8 * k)
        end
    end

    if not (st and st.dead) then
        local flick = 1 + 0.12 * math.sin(t * 30 + ent.id * 1.7) + 0.06 * math.sin(t * 47 + ent.id)

        -- Línguas de fogo atrás da bola (oposto à direção do movimento)
        if st and (st.dx ~= 0 or st.dy ~= 0) then
            local dx, dy = st.dx, st.dy
            local nx, ny = -dy, dx
            for i = 1, 3 do
                local wob = math.sin(t * 25 + i * 2.1 + ent.id) * 3
                local len = (18 + i * 5) * flick
                local w = 8 - i * 1.5
                love.graphics.setColor(1.0, 0.35 + i * 0.12, 0.05, 0.38)
                love.graphics.polygon("fill",
                    pos_x + nx * w, pos_y + ny * w,
                    pos_x - nx * w, pos_y - ny * w,
                    pos_x - dx * len + nx * wob, pos_y - dy * len + ny * wob)
            end
        end

        -- Camadas de brilho: halo -> corpo -> núcleo quente
        love.graphics.setColor(1.0, 0.30, 0.05, 0.10)
        love.graphics.circle("fill", pos_x, pos_y, 32 * flick)
        love.graphics.setColor(1.0, 0.45, 0.05, 0.20)
        love.graphics.circle("fill", pos_x, pos_y, 21 * flick)
        love.graphics.setColor(1.0, 0.60, 0.10, 0.60)
        love.graphics.circle("fill", pos_x, pos_y, 13 * flick)
        love.graphics.setColor(1.0, 0.85, 0.30, 0.95)
        love.graphics.circle("fill", pos_x, pos_y, 8.5 * flick)
        love.graphics.setColor(1.0, 1.0, 0.85, 1.0)
        love.graphics.circle("fill", pos_x, pos_y, 4.5)
    end

    love.graphics.setBlendMode("alpha")
end

local function draw_impacts(cam_x, cam_y, cx, cy)
    love.graphics.setBlendMode("add")
    for _, fx in ipairs(impact_fx) do
        local k = fx.age / fx.life
        local sx = cx + (fx.x - cam_x) * 10
        local sy = cy + (fx.y - cam_y) * 10

        love.graphics.setColor(1.0, 0.75, 0.25, (1 - k) * 0.85)
        love.graphics.circle("fill", sx, sy, 8 + 22 * k)

        love.graphics.setColor(1.0, 0.45, 0.10, 1 - k)
        love.graphics.setLineWidth(1 + 3 * (1 - k))
        love.graphics.circle("line", sx, sy, 6 + 34 * k)
        love.graphics.setLineWidth(1)

        for _, s in ipairs(fx.sparks) do
            love.graphics.setColor(1.0, 0.85, 0.35, 1 - k)
            love.graphics.circle("fill", sx + s.dx * s.speed * k, sy + s.dy * s.speed * k, 0.5 + 2 * (1 - k))
        end
    end
    love.graphics.setBlendMode("alpha")
end

local RESPAWN_DELAY = 2.0  -- segundos até tentar voltar depois de morrer
local RESPAWN_RETRY = 1.5  -- intervalo entre tentativas
local respawn_timer = nil  -- nil = vivo
local respawn_attempts = 0
local last_my_x, last_my_y = 0, 0

local function get_cam_pos()
    local my_entity = loci.get_my_entity()
    if my_entity then
        last_my_x, last_my_y = my_entity.x, my_entity.y
        return my_entity.x, my_entity.y
    end
    if not is_spectator_cli and respawn_timer then
        return last_my_x, last_my_y -- câmera fica onde morreu
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

    local random_suffix = tostring(love.math and love.math.random(1000, 9999) or math.random(1000, 9999))
    local client_name = is_spectator_cli and ("Spectator_" .. random_suffix) or ("Love2DPlayer_" .. random_suffix)
    love.window.setTitle(is_spectator_cli and "loci2d - Spectator Mode" or "loci2d - Love2D Client SDK Example")
    love.window.setMode(800, 600, { resizable = true })
    love.graphics.setDefaultFilter("nearest", "nearest")

    -- Carrega sprites Knight
    knight_img = love.graphics.newImage("assets/KnightMCAnimationsSHEET.png")
    knight_grid = anim8.newGrid(SPRITE_FRAME_W, SPRITE_FRAME_H, knight_img:getWidth(), knight_img:getHeight())
    anim_templates = {
        idle = anim8.newAnimation(knight_grid("1-8", 1), 0.15),
        walk = anim8.newAnimation(knight_grid("1-8", 4), 0.08),
    }

    -- Carrega spritesheet fireball
    fireball_img = love.graphics.newImage("assets/fireball.png")
    fireball_grid = anim8.newGrid(FIREBALL_FRAME_W, FIREBALL_FRAME_H, fireball_img:getWidth(), fireball_img:getHeight())
    fireball_anim_template = anim8.newAnimation(fireball_grid("1-" .. FIREBALL_FRAME_COUNT, 1), FIREBALL_FRAME_DURATION)

    -- Connect to the loci2d server
    loci.connect(server_ip, server_port, client_name, "../../sdks/love2d/lib/")
    last_status = is_spectator_cli and "Connected as Spectator" or ("Connected as '" .. client_name .. "'")

    -- Set up callbacks for game events
    -- (verbose per-event prints removed — they were flooding stdout every
    --  network tick and tanking the framerate; keep only real error logs)
    loci.on_entity_spawned = function(entity)
        entity_anims[entity.id] = make_entity_anim(get_entity_kind(entity))
        entity_anims[entity.id].prev_x, entity_anims[entity.id].prev_y = entity.x, entity.y
    end

    loci.on_entity_despawned = function(entity_id)
        projectile_state[entity_id] = nil
        entity_anims[entity_id] = nil
        if not is_spectator_cli and loci.my_entity_id == entity_id then
            respawn_timer = RESPAWN_DELAY
            respawn_attempts = 0
        end
        if following_entity_id == entity_id then
            following_entity_id = nil
        end
    end

    loci.on_property_changed = function(entity, key, old_val, new_val)
        -- was printing on every property change (fires per-entity, per-tick) -> huge stdout spam -> fps drop
    end

    loci.on_action_cast = function(entity, ability_id, dir_x, dir_y)
        local fx_x = entity and entity.x or 0
        local fx_y = entity and entity.y or 0
        local hit_t = first_hit_t(fx_x, fx_y, fx_x + dir_x * BEAM_LEN, fx_y + dir_y * BEAM_LEN,
            PROJECTILE_RADIUS, entity and entity.id)
        table.insert(visual_fx, {
            x = fx_x,
            y = fx_y,
            dir_x = dir_x,
            dir_y = dir_y,
            len_t = hit_t or 1,
            blocked = hit_t ~= nil,
            ability_id = ability_id,
            lifetime = 0.35,
            max_lifetime = 0.35,
        })
    end

    loci.on_match_state_changed = function(state, winner)
        match_banner = "MATCH " .. string.upper(state) .. (winner ~= "" and (" (Winner: " .. winner .. ")") or "")
        match_banner_timer = 4.0
    end

    loci.on_intent_rejected = function(reason)
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
    -- Sistema de Dash - detecta double-tap
    local current_time = love.timer.getTime()
    local last_time = last_key_time[key] or 0
    
    if current_time - last_time < DASH_DOUBLE_TAP_TIME and current_time - last_dash_time > DASH_COOLDOWN then
        -- Double-tap detectado - executar dash
        local my_entity = loci.get_my_entity()
        if my_entity then
            local dir_x, dir_y = 0, 0
            
            -- Determinar direção baseada na tecla
            if key == "w" or key == "up" then
                dir_y = -1
            elseif key == "s" or key == "down" then
                dir_y = 1
            elseif key == "a" or key == "left" then
                dir_x = -1
            elseif key == "d" or key == "right" then
                dir_x = 1
            end
            
            -- Enviar ação de dash (ability 3) com posição alvo distante
            local dash_target_x = my_entity.x + dir_x * 100
            local dash_target_y = my_entity.y + dir_y * 100
            loci.send_action(3, dash_target_x, dash_target_y)
            
            last_dash_time = current_time
        end
    end
    
    last_key_time[key] = current_time

    if is_spectator_cli then
        if key == "r" or key == "space" then
            following_entity_id = nil
            spec_cam_x, spec_cam_y = 0, 0
        end
    else
        if key == "x" or key == "k" then
            -- Explicit stop movement
            last_sent_dx, last_sent_dy = 0, 0
            loci.send_move(0, 0)
        elseif key == "e" then
            -- Ativar escudo (ability 4)
            local my_entity = loci.get_my_entity()
            if my_entity then
                loci.send_action(4, my_entity.x, my_entity.y)
            end
        end
    end
end

function love.keyreleased(key)
    -- Input polling is handled in love.update
end

function love.mousepressed(x, y, button)
    local center_x = love.graphics.getWidth() / 2
    local center_y = love.graphics.getHeight() / 2

    -- Avoid clicking through the HUD overlay
    if x >= 10 and x <= 480 and y >= 10 and y <= 165 then
        return
    end

    if is_spectator_cli then
        -- Spectator interactions: Click to follow entity, or Drag to pan camera
        local cam_x, cam_y = get_cam_pos()
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
    else
        local my_entity = loci.get_my_entity()
        if my_entity then
            local world_x = my_entity.x + (x - center_x) / 10
            local world_y = my_entity.y + (y - center_y) / 10

            if button == 1 then
                loci.send_action(1, world_x, world_y)
            elseif button == 2 then
                loci.send_action(2, world_x, world_y)
            end
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
    update_projectiles(dt)

    -- Atualiza animações das entidades
    local current_entities = loci.get_entities()
    local alive_ids = {}
    for _, entity in ipairs(current_entities) do
        alive_ids[entity.id] = true
        update_entity_anim(entity.id, entity, dt)
    end
    -- Limpa animações de entidades que sumiram
    for id in pairs(entity_anims) do
        if not alive_ids[id] then
            entity_anims[id] = nil
        end
    end

    if is_spectator_cli then
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
    else
        local my_entity = loci.get_my_entity()
        if my_entity then
            respawn_timer = nil
            -- Process robust input polling for player entity
            update_movement()
        elseif respawn_timer then
            last_sent_dx, last_sent_dy = 0, 0 -- reenvia o movimento ao renascer
            respawn_timer = respawn_timer - dt
            if respawn_timer <= 0 then
                respawn_attempts = respawn_attempts + 1
                if respawn_attempts % 2 == 1 then
                    -- tentativa 1, 3, 5...: novo join na mesma sessão
                    loci._send_intent({ join = { player_name = loci._player_name, schema_version = loci.SCHEMA_VERSION } })
                else
                    -- tentativa 2, 4, 6...: reconecta do zero
                    loci.disconnect("respawn")
                    loci.connect(server_ip, server_port, loci._player_name, "../../sdks/love2d/lib/")
                end
                respawn_timer = RESPAWN_RETRY
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
    local is_spectating = is_spectator_cli
    local cam_x, cam_y = get_cam_pos()

    -- Draw origin crosshair / grid center relative to camera
    local origin_screen_x = center_x - (cam_x * 10)
    local origin_screen_y = center_y - (cam_y * 10)
    love.graphics.setColor(0.2, 0.25, 0.35, 0.5)
    love.graphics.line(origin_screen_x - 50, origin_screen_y, origin_screen_x + 50, origin_screen_y)
    love.graphics.line(origin_screen_x, origin_screen_y - 50, origin_screen_x, origin_screen_y + 50)
    love.graphics.print("(0, 0)", origin_screen_x + 5, origin_screen_y + 5)

    -- Draw static obstacles (walls and obstacles) with alarming color
    love.graphics.setColor(1.0, 0.3, 0.1, 0.85) -- Bright orange/red alarming color

    for _, obs in ipairs(static_obstacles) do
        if obs.type == "aabb" then
            local min_screen_x = center_x + (obs.min.x - cam_x) * 10
            local min_screen_y = center_y + (obs.min.y - cam_y) * 10
            local max_screen_x = center_x + (obs.max.x - cam_x) * 10
            local max_screen_y = center_y + (obs.max.y - cam_y) * 10

            love.graphics.rectangle("fill", min_screen_x, min_screen_y,
                                    max_screen_x - min_screen_x, max_screen_y - min_screen_y)
            love.graphics.setColor(1.0, 0.6, 0.2, 0.95) -- Lighter border
            love.graphics.setLineWidth(2)
            love.graphics.rectangle("line", min_screen_x, min_screen_y,
                                    max_screen_x - min_screen_x, max_screen_y - min_screen_y)
            love.graphics.setLineWidth(1)
            love.graphics.setColor(1.0, 0.3, 0.1, 0.85) -- Reset to fill color
        elseif obs.type == "circle" then
            local center_screen_x = center_x + (obs.center.x - cam_x) * 10
            local center_screen_y = center_y + (obs.center.y - cam_y) * 10
            local screen_radius = obs.radius * 10

            love.graphics.circle("fill", center_screen_x, center_screen_y, screen_radius)
            love.graphics.setColor(1.0, 0.6, 0.2, 0.95) -- Lighter border
            love.graphics.setLineWidth(2)
            love.graphics.circle("line", center_screen_x, center_screen_y, screen_radius)
            love.graphics.setLineWidth(1)
            love.graphics.setColor(1.0, 0.3, 0.1, 0.85) -- Reset to fill color
        end
    end

    -- Render all entities from the authoritative world state
    local current_entities = loci.get_entities()
    for _, entity in ipairs(current_entities) do
        local pos_x = center_x + (entity.x - cam_x) * 10
        local pos_y = center_y + (entity.y - cam_y) * 10

        local st = entity_anims[entity.id]
        local is_fireball = st ~= nil and st.kind == "fireball"

        if is_fireball then
            -- Render fireball with sprite animation
            love.graphics.setColor(1, 1, 1)
            st.anim:draw(fireball_img, pos_x, pos_y, st.rotation, FIREBALL_SCALE, FIREBALL_SCALE,
                FIREBALL_FRAME_W / 2, FIREBALL_FRAME_H / 2)
        else
            -- Render player with sprite animation
            local team = entity.team
            if team == 1 or team == "1" then
                love.graphics.setColor(1, 0.75, 0.75) -- Team 1 Red tint
            elseif team == 2 or team == "2" then
                love.graphics.setColor(0.75, 0.75, 1) -- Team 2 Blue tint
            else
                love.graphics.setColor(1, 1, 1)
            end

            if entity:is_local_player() then
                love.graphics.setColor(0.75, 0.9, 1) -- Local Player tint
            end

            -- Draw Knight sprite (idle/walk, facing left/right)
            if st then
                local sx = SPRITE_SCALE * st.facing
                st.anim:draw(knight_img, pos_x, pos_y, 0, sx, SPRITE_SCALE, SPRITE_FRAME_W / 2, SPRITE_FRAME_H / 2)
            end
            love.graphics.setColor(1, 1, 1)

            -- Visual effect for Slow (blue tint around player)
            if entity.properties and entity.properties.status_slow == "true" then
                love.graphics.setColor(0.3, 0.6, 1.0, 0.4)
                love.graphics.circle("fill", pos_x, pos_y, 20)
                love.graphics.setColor(1, 1, 1)
                love.graphics.circle("line", pos_x, pos_y, 20)
            end

            -- Visual effect for Dash (yellow tint around player)
            if entity.properties and entity.properties.dash_active == "true" then
                love.graphics.setColor(1.0, 0.8, 0.2, 0.5)
                love.graphics.circle("fill", pos_x, pos_y, 22)
                love.graphics.setColor(1, 1, 1)
                love.graphics.circle("line", pos_x, pos_y, 22)
            end

            -- Visual effect for Shield (cyan tint around player)
            if entity.properties and entity.properties.shield_active == "true" then
                love.graphics.setColor(0.2, 0.8, 0.9, 0.5)
                love.graphics.circle("fill", pos_x, pos_y, 24)
                love.graphics.setColor(1, 1, 1)
                love.graphics.circle("line", pos_x, pos_y, 24)
                love.graphics.circle("line", pos_x, pos_y, 28)
            end

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

            -- Draw HP if it exists
            local hp = entity.hp
            if hp and (not is_spectating or following_entity_id ~= entity.id) then
                love.graphics.setColor(1, 0.2, 0.2)
                love.graphics.print("HP: " .. tostring(hp), pos_x - 20, pos_y + 20)
            end
        end
    end

    -- Explosões de impacto das skills
    draw_impacts(cam_x, cam_y, center_x, center_y)

    -- Render transient action visual effects (Phase 6.5.3-1)
    for _, fx in ipairs(visual_fx) do
        local alpha = math.max(0, fx.lifetime / fx.max_lifetime)
        local start_x = center_x + (fx.x - cam_x) * 10
        local start_y = center_y + (fx.y - cam_y) * 10
        local end_x = start_x + (fx.dir_x * 45 * fx.len_t)
        local end_y = start_y + (fx.dir_y * 45 * fx.len_t)

        if fx.ability_id == 1 then
            love.graphics.setColor(1, 0.85, 0.2, alpha) -- Ability 1: Yellow Beam
            love.graphics.setLineWidth(3)
        else
            love.graphics.setColor(0.85, 0.2, 1, alpha) -- Ability 2: Purple Beam
            love.graphics.setLineWidth(5)
        end
        love.graphics.line(start_x, start_y, end_x, end_y)

        -- Add green ball at the end of the beam (fireball visual)
        if not fx.blocked then
            love.graphics.setBlendMode("add")
            love.graphics.setColor(1.0, 0.55, 0.1, alpha * 0.5)
            love.graphics.circle("fill", end_x, end_y, 14 * alpha)
            love.graphics.setColor(1.0, 0.9, 0.5, alpha)
            love.graphics.circle("fill", end_x, end_y, 6 * alpha)
            love.graphics.setBlendMode("alpha")
        end
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

    -- Aviso de morte / respawn
    if not is_spectating and not my_entity and respawn_timer then
        love.graphics.setColor(0, 0, 0, 0.45)
        love.graphics.rectangle("fill", 0, center_y - 30, love.graphics.getWidth(), 60)
        love.graphics.setColor(1, 0.3, 0.3)
        love.graphics.printf(string.format("VOCÊ MORREU - respawn em %.1fs", math.max(0, respawn_timer)),
            0, center_y - 8, love.graphics.getWidth(), "center")
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
        if my_entity then
            love.graphics.print(string.format("Player Entity: %s (id=%d)", my_entity.blueprint or loci._player_name, my_entity.id or 0), 20, 72)
        else
            love.graphics.print("Player Entity: Connecting / Waiting for spawn...", 20, 72)
        end
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


