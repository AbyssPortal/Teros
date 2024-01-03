mod engine;

use std::{env, io::stdin, io::stdout, sync::Arc};

use rust_chess::chess::{
    self,
    chess::{make_board_from_fen, Board, Color},
};
use std::thread;
use text_io::read;

use crate::engine::teros_engine::{
    InterestEvaluationWeights, MinimaxSettings, StaticEvaluationWeights,
};

const THREAD_COUNT: usize = 32;

const MAX_MINIMAX_DEPTH: i32 = 2;

fn main() {
    let args: Vec<String> = env::args().collect();
    let setup = args.iter().any(|arg| arg == "-su");

    let stdin = stdin();
    let board = match setup && yes_or_no("use fen?") {
        true => {
            println!("enter fen");
            loop {
                let mut text = String::new();
                stdin.read_line(&mut text).unwrap();
                match make_board_from_fen(&text) {
                    Ok(board) => break board,
                    Err(err) => {
                        println!("Error! Please try again! ({:?})", err)
                    }
                }
            }
        }
        false => Board::new(),
    };

    let turns_to_eval = vec![
        match !setup || yes_or_no("Eval for white?") {
            true => Some(Color::White),
            false => None,
        },
        match !setup || yes_or_no("Eval for black?") {
            true => Some(Color::Black),
            false => None,
        },
    ];

    let turns_to_play = vec![
        match turns_to_eval.contains(&Some(Color::White)) && setup && yes_or_no("Play for white?") {
            true => Some(Color::White),
            false => None,
        },
        match turns_to_eval.contains(&Some(Color::Black)) && setup && yes_or_no("Play for black?") {
            true => Some(Color::Black),
            false => None,
        },
    ];

    let mut engine = engine::teros_engine::Engine::new(
        board,
        StaticEvaluationWeights::new(),
        InterestEvaluationWeights::new(),
        MinimaxSettings::new(),
    );

    let max_pondering: Option<usize> = match setup && yes_or_no("Limit pondering?") {
        false => None,
        true => {
            println!("how much?");
            Some(read!())
        }
    };

    let pgn_mode = setup && yes_or_no("pgn only mode?");

    // engine.print_tree(10);
    let mut stdout = stdout();
    const START_EVAL_TURN: i32 = 0;
    let mut i = 1;
    let mut move_number = 1;
    loop {
        if turns_to_eval.contains(&Some(engine.get_board().get_turn())) && i >= START_EVAL_TURN {
            // ...

            let (stop_sender, stop_reciever) = std::sync::mpsc::channel();

            engine = match max_pondering {
                Some(max_pondering_num) => {
                    if !pgn_mode {
                        println!("PONDERING!!!! (until done as much as you told me)");
                    }

                    let res =
                        engine.multi_thread_think_next_num_moves(THREAD_COUNT, max_pondering_num);

                    if !pgn_mode {
                        println!("EVALUATING!!!!");
                    }
                    res
                }
                None => {
                    if !pgn_mode {
                        println!("PONDERING!!!! (enter any value to stop)");
                    }

                    let thread_handle = thread::spawn(move || {
                        engine.multi_thread_think_next_moves_until_stop(THREAD_COUNT, stop_reciever)
                    });

                    let _: String = read!();

                    stop_sender.send(()).unwrap();

                    let res = thread_handle.join().unwrap();

                    if !pgn_mode {
                        println!("PONDERED {} TIMES!!!!", res.1);
                        println!("EVALUATING!!!!");
                    }
                    res.0
                }
            };
            let engine_arc = Arc::new(engine);
            let eval = engine_arc
                .clone()
                .parallel_eval_and_best_move(MAX_MINIMAX_DEPTH);
            engine = Arc::try_unwrap(engine_arc).unwrap();
            // engine.print_tree(10);
            if !pgn_mode {
                println!(
                    "BALANCE IS {}. I LIKE THE MOVE {}",
                    eval.0,
                    match eval.1 {
                        Some(chess_move) => chess_move.name(engine.get_board()).unwrap(),
                        None => String::from("THERE ARE NO MOVES"),
                    }
                );
            }
            if turns_to_play.contains(&Some(engine.get_board().get_turn())) {
                match eval.1 {
                    Some(chess_move) => {
                        make_engine_move_and_print(
                            pgn_mode,
                            &mut engine,
                            &mut move_number,
                            chess_move,
                            &mut stdout,
                        )
                        .unwrap();
                        continue;
                    }
                    None => {
                        break;
                    }
                }
            } else {
                engine.get_board().print_board(&mut stdout).unwrap();
            }
        }
        if !pgn_mode {
            println!("U THINK NOW!!!!");
        }
        loop {
            let chess_move_string: String = read!();
            let chess_move = engine.get_board().interpret_move(&chess_move_string);
            match chess_move {
                Ok(chess_move) => match make_engine_move_and_print(
                    pgn_mode,
                    &mut engine,
                    &mut move_number,
                    chess_move,
                    &mut stdout,
                ) {
                    Ok(_) => {
                        break;
                    }
                    Err(err) => {
                        println!("YOU FORGOT!!!! ({:?})", err)
                    }
                },
                Err(err) => {
                    println! {"NO CHEATING!!!! ({:?})", err }
                }
            }
        }
        i += 1;
    }

    // println!("{:#?}", engine); // Debug print the engine variable
}

fn make_engine_move_and_print(
    pgn_mode: bool,
    engine: &mut engine::teros_engine::Engine,
    move_number: &mut i32,
    chess_move: chess::chess::ChessMove,
    stdout: &mut std::io::Stdout,
) -> Result<(), engine::teros_engine::EngineError> {
    if pgn_mode {
        match engine.get_board().get_turn() {
            Color::White => print!(
                "{}. {}",
                move_number,
                chess_move.name(engine.get_board()).unwrap()
            ),
            Color::Black => {
                print!(" {}\n", chess_move.name(engine.get_board()).unwrap());
                *move_number += 1;
            }
        }
    } else {
        engine.get_board().print_board(stdout).unwrap();
    }
    engine.make_move(&chess_move)
}

fn yes_or_no(question: &str) -> bool {
    loop {
        println!("{} (y/n)", question);
        let letter: char = read!();
        match letter.to_ascii_lowercase() {
            'y' => return true,
            'n' => return false,
            _ => {}
        }
    }
}
