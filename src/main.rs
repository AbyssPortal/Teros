mod engine;
fn main() {
    let mut engine = engine::teros_engine::Engine::new();
    engine.print_tree(10);
    println!("THINKING!!!!");
    for _ in 0..10 {
        engine.think_next_move().unwrap();
    }
    engine.print_tree(10);

    // println!("{:#?}", engine); // Debug print the engine variable
}
