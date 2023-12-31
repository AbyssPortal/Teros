mod engine;

use std::io::stdout;

use text_io::read;
fn main() {
    let mut engine = engine::teros_engine::Engine::new();

    engine.print_tree(10);
    let mut stdout = stdout();
    const START_EVAL_TURN: i32= 74;
    let mut i = 1;
    loop {
        if engine.get_board().get_turn() == rust_chess::chess::chess::Color::White && i >= START_EVAL_TURN {
            println!("PONDERING!!!!");
            for _ in 0..10000 {
                engine.think_next_move().unwrap();
            }
            println!("EVALUATING!!!!");
            let eval = engine.eval_and_best_move();

            println!(
                "BALANCE IS {}. I LIKE THE MOVE {}",
                eval.0,
                match eval.1 {
                    Some(chess_move) => chess_move.name(engine.get_board()).unwrap(),
                    None => String::from("THERE ARE NO MOVES"),
                }
            );
        }
        // engine.print_tree(10);
        engine.get_board().print_board(&mut stdout).unwrap();
        println!("U THINK NOW!!!!");
        let chess_move_string: String = read!();
        engine.interpret_and_make_move(&chess_move_string).unwrap();
        i += 1;
    }

    // println!("{:#?}", engine); // Debug print the engine variable
}
