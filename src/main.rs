mod engine;
fn main() {
    let mut engine = engine::teros_engine::Engine::new();
    engine.print_tree(3);
    println!("THINKING!!!!");
    engine.think_next_move().unwrap();
    engine.think_next_move().unwrap();
    engine.print_tree(3);

    // println!("{:#?}", engine); // Debug print the engine variable
}
