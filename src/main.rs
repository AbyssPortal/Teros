mod engine;


use text_io::read;
fn main() {

    let mut engine = engine::teros_engine::Engine::new();
    engine.print_tree(10);
    println!("THINKING!!!!");
    for _ in 0..10 {
        engine.think_next_move().unwrap();
    }
    engine.print_tree(10);
    println!("MOVING!!!!");
    let chess_move_string : String = read!();
    engine.interpret_and_make_move(&chess_move_string).unwrap();
    engine.print_tree(10);
    // println!("{:#?}", engine); // Debug print the engine variable
}
