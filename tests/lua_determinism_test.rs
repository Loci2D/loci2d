use loci2d::world::instance::Instance;

#[test]
fn test_lua_determinism_prng_parity() {
    let mut inst1 = Instance::new(1, 30, 10, 1337);
    let mut inst2 = Instance::new(2, 30, 10, 1337);

    let script = r#"
        function on_tick(tick)
            return math.random(1, 100), math.random(), math.random(50, 60)
        end
    "#;

    inst1.load_script(script).unwrap();
    inst2.load_script(script).unwrap();

    let globals1 = inst1.script_engine.lua().globals();
    let on_tick_1: mlua::Function = globals1.get("on_tick").unwrap();

    let globals2 = inst2.script_engine.lua().globals();
    let on_tick_2: mlua::Function = globals2.get("on_tick").unwrap();

    for i in 1..=50 {
        let res1: (i32, f64, i32) = on_tick_1.call(i).unwrap();
        let res2: (i32, f64, i32) = on_tick_2.call(i).unwrap();

        assert_eq!(res1.0, res2.0, "Mismatch at tick {}", i);
        assert_eq!(res1.1, res2.1, "Mismatch at tick {}", i);
        assert_eq!(res1.2, res2.2, "Mismatch at tick {}", i);
    }
}

#[test]
fn test_lua_determinism_different_seeds() {
    let mut inst1 = Instance::new(1, 30, 10, 100);
    let mut inst2 = Instance::new(2, 30, 10, 200);

    let script = r#"
        function get_rand()
            return math.random(1, 1000000)
        end
    "#;

    inst1.load_script(script).unwrap();
    inst2.load_script(script).unwrap();

    let globals1 = inst1.script_engine.lua().globals();
    let get_rand_1: mlua::Function = globals1.get("get_rand").unwrap();

    let globals2 = inst2.script_engine.lua().globals();
    let get_rand_2: mlua::Function = globals2.get("get_rand").unwrap();

    let r1: i32 = get_rand_1.call(()).unwrap();
    let r2: i32 = get_rand_2.call(()).unwrap();
    
    assert_ne!(r1, r2, "Different seeds should produce different sequences");
}

#[test]
fn test_lua_determinism_no_pairs() {
    let mut inst = Instance::new(1, 30, 10, 42);
    
    let script = r#"
        local t = {a=1, b=2}
        for k, v in pairs(t) do
            -- should fail here
        end
    "#;

    let res = inst.load_script(script);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("attempt to call a nil value (global 'pairs')"));
}

#[test]
fn test_lua_script_hash() {
    let mut inst = Instance::new(1, 30, 10, 42);
    let script1 = "function on_init() end";
    let script2 = "function on_tick() end";

    inst.load_script(script1).unwrap();
    let hash1 = inst.script_hash.clone();
    assert!(!hash1.is_empty(), "Script hash must be populated");

    inst.load_script(script2).unwrap();
    let hash2 = inst.script_hash.clone();
    
    assert_ne!(hash1, hash2, "Different scripts must have different hashes");
}
