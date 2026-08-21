use loci2d::scripting::ScriptEngine;
use loci2d::world::instance::Instance;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_io_library_disabled() {
    let engine = ScriptEngine::new().expect("Failed to initialize ScriptEngine");

    // Accessing io should fail (io is nil)
    let result = engine.load_script(
        r#"
        local file = io.open("sandbox_test.txt", "w")
        "#,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("attempt to index a nil value") || err.contains("global 'io'"),
        "Unexpected error output: {err}"
    );
}

#[test]
fn test_os_library_disabled() {
    let engine = ScriptEngine::new().expect("Failed to initialize ScriptEngine");

    // Accessing os library should fail (os is nil)
    let result = engine.load_script(
        r#"
        os.execute("echo hacked")
        "#,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("attempt to index a nil value") || err.contains("global 'os'"),
        "Unexpected error output: {err}"
    );
}

#[test]
fn test_package_and_debug_disabled() {
    let engine = ScriptEngine::new().expect("Failed to initialize ScriptEngine");

    let package_res = engine.load_script("local p = package.loaded");
    assert!(package_res.is_err());

    let debug_res = engine.load_script("local d = debug.getinfo(1)");
    assert!(debug_res.is_err());
}

#[test]
fn test_safe_standard_libraries_functional() {
    let engine = ScriptEngine::new().expect("Failed to initialize ScriptEngine");

    let script = r#"
        local val = math.floor(10.7)
        assert(val == 10)

        local str = string.upper("loci2d")
        assert(str == "LOCI2D")

        local t = {1, 2, 3}
        table.insert(t, 4)
        assert(#t == 4)
    "#;

    let result = engine.load_script(script);
    assert!(result.is_ok(), "Safe script failed: {:?}", result.err());
}

#[test]
fn test_dos_protection_infinite_loop() {
    // Set a small instruction limit for rapid test execution
    let engine = ScriptEngine::new_with_instruction_limit(1_000)
        .expect("Failed to initialize ScriptEngine with low instruction limit");

    let infinite_loop_script = r#"
        while true do
            -- busy loop
        end
    "#;

    let result = engine.load_script(infinite_loop_script);
    assert!(result.is_err(), "Infinite loop script should have been interrupted!");
    let err = result.unwrap_err();
    assert!(
        err.contains("Execution limit exceeded"),
        "Error message should mention limit exceeded: {err}"
    );
}

#[test]
fn test_dos_protection_resets_per_invocation() {
    // Instruction limit of 500
    let engine = ScriptEngine::new_with_instruction_limit(500)
        .expect("Failed to initialize ScriptEngine");

    // Execute scripts that each consume 300 instructions (total 600 > 500 across two calls)
    let normal_script = r#"
        local sum = 0
        for i = 1, 30 do
            sum = sum + i
        end
    "#;

    // First call should succeed
    let res1 = engine.load_script(normal_script);
    assert!(res1.is_ok(), "First script execution failed: {:?}", res1.err());

    // Second call should also succeed because counter resets per invocation
    let res2 = engine.load_script(normal_script);
    assert!(res2.is_ok(), "Second script execution failed due to unreset counter: {:?}", res2.err());
}

#[test]
fn test_instance_script_loading() {
    let mut instance = Instance::new(1, 30, 10);

    // Test inline script loading
    let inline_res = instance.load_script("test_var = 100");
    assert!(inline_res.is_ok());

    // Test file script loading via temp file
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(temp_file, "file_var = 200").expect("Failed to write to temp file");

    let file_res = instance.load_script_from_file(temp_file.path());
    assert!(file_res.is_ok());

    // Verify globals were set in instance script engine
    let lua = instance.script_engine.lua();
    let test_var: i32 = lua.globals().get("test_var").expect("test_var not set");
    let file_var: i32 = lua.globals().get("file_var").expect("file_var not set");

    assert_eq!(test_var, 100);
    assert_eq!(file_var, 200);
}
