pub const PASSTHROUGH_MOVEMENT_SCRIPT: &str = r#"
    function on_move_intent(id, x, y)
        Loci.Commands.set_velocity(id, Loci.Vector2(x, y))
    end
    function on_nav_intent(id, x, y)
        Loci.Commands.set_navigation_target(id, Loci.Vector2(x, y))
    end
"#;
