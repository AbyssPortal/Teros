mod engine;

use std::{io::{stdout, Read}, io::stdin};

use rust_chess::chess::chess::{Board, make_board_from_fen, Color};
use text_io::read;

use crate::engine::teros_engine::{
    InterestEvaluationWeights, MinimaxSettings, StaticEvaluationWeights,
};


fn main() {
    let stdin = stdin();
    let board = match yes_or_no("use fen?") {
        true =>  {
            println!("enter fen");
            loop {
                let mut text = String::new();
                stdin.read_line(&mut text).unwrap();
                match make_board_from_fen(&text) {
                    Ok(board) => break board,
                    Err(err) => {println!("Error! Please try again! ({:?})", err)} 
                }
            }
        }
        false => Board::new()
    };

    let turns_to_eval = vec![match yes_or_no("Eval for white?") {
        true => Some(Color::White),
        false => None,
    },
    match yes_or_no("Eval for black?") {
        true => Some(Color::Black),
        false => None,
    }];

    let turns_to_play = vec![match turns_to_eval.contains(&Some(Color::White)) && yes_or_no("Play for white?") {
        true => Some(Color::White),
        false => None,
    },
    match turns_to_eval.contains(&Some(Color::Black)) && yes_or_no("Play for black?") {
        true => Some(Color::Black),
        false => None,
    }];

    let mut engine = engine::teros_engine::Engine::new(
        board,
        StaticEvaluationWeights::new(),
        InterestEvaluationWeights::new(),
        MinimaxSettings::new(),
    );

    // engine.print_tree(10);
    let mut stdout = stdout();
    const START_EVAL_TURN: i32 = 0;
    const ALLOWED_MOVE_THINKUS: i32 = 10_000;
    let mut i = 1;
    loop {
        if turns_to_eval.contains(&Some(engine.get_board().get_turn()))
            && i >= START_EVAL_TURN
        {
            println!("PONDERING!!!!");
            for _ in 0..ALLOWED_MOVE_THINKUS {
                match engine.think_next_move() {
                    Ok(_) => {},
                    Err(engine::teros_engine::EngineError::NoValidMovesErrror) => {
                        break;
                    }
                    Err(err) => {
                        panic!("{:?}", err);
                    }
                }
            }
            println!("EVALUATING!!!!");
            let eval = engine.eval_and_best_move();
            // engine.print_tree(10);
            println!(
                "BALANCE IS {}. I LIKE THE MOVE {}",
                eval.0,
                match eval.1 {
                    Some(chess_move) => chess_move.name(engine.get_board()).unwrap(),
                    None => String::from("THERE ARE NO MOVES"),
                }
            );
            if turns_to_play.contains(&Some(engine.get_board().get_turn())) {
                match eval.1 {
                    Some(chess_move) => {
                        engine.get_board().print_board(&mut stdout).unwrap();
                        engine.make_move(&chess_move).unwrap();
                        continue;
                    },
                    None => {break;}
                }
            }
        }
        engine.get_board().print_board(&mut stdout).unwrap();
        println!("U THINK NOW!!!!");
        loop {
            let chess_move_string: String = read!();
            let chess_move = engine.get_board().interpret_move(&chess_move_string);
            match chess_move {
                Ok(chess_move) => match engine.make_move(&chess_move) {
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


fn yes_or_no(question: &str) -> bool {
    loop {
        println!("{} (y/n)", question);
        let letter : char = read!();
        match letter.to_ascii_lowercase() {
            'y' => return true,
            'n' => return false,
            _ => {},
        }
    }
}