mod engine;

use std::{
    io::stdin,
    io::stdout,
};

use rust_chess::chess::{
    self,
    chess::{make_board_from_fen, Board, Color},
};
use std::thread;
use text_io::read;

use crate::engine::teros_engine::{
    InterestEvaluationWeights, MinimaxSettings, StaticEvaluationWeights,
};

fn main() {
    let stdin = stdin();
    let board = match yes_or_no("use fen?") {
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
        match yes_or_no("Eval for white?") {
            true => Some(Color::White),
            false => None,
        },
        match yes_or_no("Eval for black?") {
            true => Some(Color::Black),
            false => None,
        },
    ];

    let turns_to_play = vec![
        match turns_to_eval.contains(&Some(Color::White)) && yes_or_no("Play for white?") {
            true => Some(Color::White),
            false => None,
        },
        match turns_to_eval.contains(&Some(Color::Black)) && yes_or_no("Play for black?") {
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

    let max_pondering: Option<i32> = match yes_or_no("Limit pondering?") {
        false => None,
        true => {
            println!("how much?");
            Some(read!())
        }
    };

    let pgn_mode = yes_or_no("pgn only mode?");

    // engine.print_tree(10);
    let mut stdout = stdout();
    const START_EVAL_TURN: i32 = 0;
    let mut i = 1;
    let mut move_number = 1;
    loop {
        if turns_to_eval.contains(&Some(engine.get_board().get_turn())) && i >= START_EVAL_TURN {
            if !pgn_mode {
                println!("PONDERING!!!! (enter any value to stop)");
            }
            // ...

            let (stop_sender, stop_reciever) = std::sync::mpsc::channel();

            let thread_handle = thread::spawn(move || {
                let mut i = 0;
                loop {
                    match stop_reciever.try_recv() {
                        Ok(_) => break,
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            match engine.think_next_move() {
                                Ok(_) => {}
                                Err(engine::teros_engine::EngineError::NoValidMovesErrror) => {
                                    break;
                                }
                                Err(err) => {
                                    panic!("{:?}", err);
                                }
                            }
                        }
                        Err(err) => {
                            panic!("{:?}", err);
                        }
                    }
                    i += 1;
                    if let Some(num) = max_pondering {
                        if i > num {
                            break;
                        }
                    }
                }
                (engine, i)
            });

            if max_pondering.is_none() {
                let _: String = read!();
                stop_sender.send(()).unwrap();
            }

            let res = thread_handle.join().unwrap();
            engine = res.0;
            let i = res.1;

            if !pgn_mode {
                println!("PONDERED {} TIMES!!!!", i);
                println!("EVALUATING!!!!");
            }
            let eval = engine.eval_and_best_move();
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
