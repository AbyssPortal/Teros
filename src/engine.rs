extern crate rust_chess;

pub mod teros_engine {
    use std::{collections::BinaryHeap, f32::INFINITY};

    use ordered_float::NotNan;
    use std::collections::BTreeMap;

    use rust_chess::chess::chess::*;

    fn piece_worth(piece: PieceKind) -> NotNan<f32> {
        match piece {
            PieceKind::Pawn => NotNan::new(1.0).unwrap(),
            PieceKind::Knight => NotNan::new(3.0).unwrap(),
            PieceKind::Bishop => NotNan::new(3.0).unwrap(),
            PieceKind::Rook => NotNan::new(5.0).unwrap(),
            PieceKind::Queen => NotNan::new(9.0).unwrap(),
            PieceKind::King => NotNan::new(INFINITY).unwrap(),
        }
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]

    struct ValuedMoveLocation {
        valued_move: ValuedChessMove,
        location: Vec<ChessMove>,
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
    struct ValuedChessMove {
        value: NotNan<f32>, // probably precise enough
        chess_move: ChessMove,
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
    struct MoveTree {
        board_state: Board,
        moves: BTreeMap<ChessMove, Option<MoveTree>>,
    }

    impl MoveTree {
        pub fn print_tree(&self, depth: i32, max_depth: i32) {
            if (depth > max_depth) {
                return;
            }
            for (chess_move, tree_option) in self.moves.iter() {
                for i in 0..depth {
                    print!("|");
                }
                print!("-");
                println!("{}", chess_move.name());
                if let Some(tree) = tree_option {
                    tree.print_tree(depth + 1, max_depth);
                }
            }
        }
    }

    #[derive(Debug)]
    pub struct Engine {
        moves: BinaryHeap<ValuedMoveLocation>,
        move_tree: MoveTree,
    }

    #[derive(Clone, Copy, Debug)]
    pub enum EngineError {
        InvalidLocationError,
        NoValidMovesErrror,
    }

    impl<'a> Engine {
        pub fn new() -> Engine {
            let mut res = Engine {
                moves: BinaryHeap::new(),
                move_tree: MoveTree {
                    moves: BTreeMap::new(),
                    board_state: Board::new(),
                },
            };
            res.generate_all_moves(vec![]).unwrap();
            res
        }

        

        //go to a branch specified by the list of moves in location.
        fn go_to_location(
            &'a mut self,
            location: &Vec<ChessMove>,
        ) -> Result<&'a mut MoveTree, EngineError> {
            let mut current_tree = &mut self.move_tree;
            for chess_move in location {
                match current_tree.moves.get_mut(chess_move) {
                    Some(Some(child_tree)) => {
                        current_tree = child_tree;
                    }
                    Some(None) => {
                        return Err(EngineError::InvalidLocationError);
                    }
                    None => {
                        return Err(EngineError::InvalidLocationError);
                    }
                }
            }
            Ok(current_tree)
        }

        fn generate_all_moves(&mut self, location: Vec<ChessMove>) -> Result<(), EngineError> {
            let tree = self.go_to_location(&location)?;
            for i in 0..BOARD_SIZE {
                for j in 0..BOARD_SIZE {
                    match tree.board_state.generate_moves(i, j) {
                        Ok(moves) => {
                            for chess_move in moves {
                                let mut new_board = tree.board_state.clone();
                                match new_board.make_legal_move(chess_move) {
                                    Ok(()) => {
                                        tree.moves.insert(
                                            chess_move,
                                            Some(MoveTree {
                                                board_state: new_board,
                                                moves: BTreeMap::new(),
                                            }),
                                        );
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                        Err(BoardError::NoPieceError | BoardError::WrongTurnError) => {}
                        Err(_) => {
                            panic!("what");
                        }
                    }
                }
            }
            let mut new_moves = Vec::new();

            for (chess_move, _) in tree.moves.iter() {
                new_moves.push(ValuedMoveLocation {
                    valued_move: ValuedChessMove {
                        chess_move: chess_move.clone(),
                        value: Engine::evaluate_interest(chess_move, &tree.board_state).unwrap(),
                    },
                    location: location.clone(),
                });
            }
            for new_move in new_moves {
                self.moves.push(new_move);
            }
            Ok(())
        }

        pub fn print_tree(&self, depth: i32) {
            self.move_tree.print_tree(0, depth);
        }

        pub fn print_moves(&mut self) {
            let mut temp_heap = self.moves.clone();

            while let Some(valued_move) = temp_heap.pop() {
                println!("{:#?}", valued_move);
            }
        }

        pub fn think_next_move(&mut self) -> Result<(), EngineError> {
            let next_move = self.moves.pop().ok_or(EngineError::NoValidMovesErrror)?;
            let mut location = next_move.location;
            location.push(next_move.valued_move.chess_move);
            self.generate_all_moves(location)?;
            Ok(())
        }

        fn evaluate_interest(
            chess_move: &ChessMove,
            board: &Board,
        ) -> Result<NotNan<f32>, BoardError> {
            use ChessMove::*;
            //temporary
            Ok(match chess_move {
                Normal(normal_move) => piece_worth(
                    board
                        .get_piece(normal_move.initial_row, normal_move.initial_col)?
                        .ok_or(BoardError::NoPieceError)?
                        .kind,
                ),
                Promotion(_, piece_kind) => piece_worth(*piece_kind),
                Castling(_) => NotNan::new(5.0).unwrap(),
            })
        }
    }
}
