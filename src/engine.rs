extern crate rust_chess;

#[allow(dead_code)]
pub mod teros_engine {
    use std::{
        collections::{BinaryHeap, VecDeque},
        f32::INFINITY,
        sync::{mpsc::Receiver, Arc, Mutex},
        thread,
    };

    use ordered_float::NotNan;
    use std::cmp::Ordering;
    use std::collections::BTreeMap;
    use std::fmt;

    use rust_chess::chess::chess::*;

    fn piece_worth_king_inf(piece: PieceKind) -> NotNan<f32> {
        match piece {
            PieceKind::Pawn => NotNan::new(1.0).unwrap(),
            PieceKind::Knight => NotNan::new(3.0).unwrap(),
            PieceKind::Bishop => NotNan::new(3.0).unwrap(),
            PieceKind::Rook => NotNan::new(5.0).unwrap(),
            PieceKind::Queen => NotNan::new(9.0).unwrap(),
            PieceKind::King => NotNan::new(INFINITY).unwrap(),
        }
    }

    fn piece_worth_king_zero(piece: PieceKind) -> NotNan<f32> {
        match piece {
            PieceKind::Pawn => NotNan::new(1.0).unwrap(),
            PieceKind::Knight => NotNan::new(3.0).unwrap(),
            PieceKind::Bishop => NotNan::new(3.0).unwrap(),
            PieceKind::Rook => NotNan::new(5.0).unwrap(),
            PieceKind::Queen => NotNan::new(9.0).unwrap(),
            PieceKind::King => NotNan::new(0.0).unwrap(),
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub enum Eval {
        Numeric(NotNan<f32>),
        MateIn(Color, i32),
    }

    impl Eval {
        fn increase_mate_counter(self) -> Eval {
            match self {
                Eval::Numeric(_) => self,
                Eval::MateIn(color, counter) => Eval::MateIn(color, counter + 1),
            }
        }
    }

    impl fmt::Display for Eval {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Eval::Numeric(value) => write!(f, "{}", value),
                Eval::MateIn(color, value) => {
                    write!(
                        f,
                        "{}M{}",
                        match color {
                            Color::Black => '-',
                            Color::White => '+',
                        },
                        value
                    )
                }
            }
        }
    }
    #[test]
    fn test_eval() {
        let _m5 = Eval::MateIn(Color::Black, 5);
        let _m1 = Eval::MateIn(Color::Black, 1);
        let m5 = Eval::MateIn(Color::White, 5);
        let m1 = Eval::MateIn(Color::White, 1);
        let one = Eval::Numeric(NotNan::new(1.0).unwrap());
        let _one = Eval::Numeric(NotNan::new(-1.0).unwrap());
        assert!(_m5 > _m1);
        assert!(m5 < m1);
        assert!(_m5 < m1);
        assert!(m5 > _m1);
        assert!(m1 > one);
        assert!(one > _one);
        assert!(_one > _m1);
    }

    impl Ord for Eval {
        fn cmp(&self, other: &Self) -> Ordering {
            match (self, other) {
                (Eval::Numeric(value1), Eval::Numeric(value2)) => value1.cmp(value2),
                (Eval::MateIn(color1, value1), Eval::MateIn(color2, value2)) => {
                    match (color1, color2) {
                        (Color::Black, Color::White) => Ordering::Less,
                        (Color::White, Color::Black) => Ordering::Greater,
                        (Color::White, Color::White) => value1.cmp(value2).reverse(),
                        (Color::Black, Color::Black) => value1.cmp(value2),
                    }
                }
                (Eval::Numeric(_), Eval::MateIn(color, _)) => match color {
                    Color::Black => Ordering::Greater,
                    Color::White => Ordering::Less,
                },
                (Eval::MateIn(_, _), Eval::Numeric(_)) => other.cmp(self).reverse(),
            }
        }
    }

    impl PartialOrd for Eval {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone)]

    struct ValuedMoveLocation {
        valued_move: ValuedChessMove,
        location: VecDeque<ChessMove>,
        depth_cost: NotNan<f32>,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord)]
    struct ValuedChessMove {
        value: NotNan<f32>, // probably precise enough
        chess_move: ChessMove,
    }

    impl ValuedMoveLocation {
        fn value_accounted_for_distance(&self) -> NotNan<f32> {
            const DEPTH_COST: f32 = 25.0;
            self.valued_move.value - self.depth_cost * self.location.len() as f32
        }
    }

    impl Ord for ValuedMoveLocation {
        fn cmp(&self, other: &Self) -> Ordering {
            match self
                .value_accounted_for_distance()
                .cmp(&other.value_accounted_for_distance())
            {
                Ordering::Equal => match self.valued_move.cmp(&other.valued_move) {
                    Ordering::Equal => match self.location.cmp(&other.location) {
                        Ordering::Equal => {
                            assert_eq!(self, other);
                            Ordering::Equal
                        }
                        Ordering::Greater => Ordering::Greater,
                        Ordering::Less => Ordering::Less,
                    },
                    Ordering::Greater => Ordering::Greater,
                    Ordering::Less => Ordering::Less,
                },
                Ordering::Greater => Ordering::Greater,
                Ordering::Less => Ordering::Less,
            }
        }
    }

    impl PartialOrd for ValuedMoveLocation {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
    struct MoveTree {
        board_state: Board,
        moves: BTreeMap<ChessMove, MoveTree>,
    }

    impl MoveTree {
        pub fn is_leaf(&self) -> bool {
            return self.moves.len() == 0;
        }

        pub fn print_tree(&self, depth: i32, max_depth: i32) {
            if depth > max_depth {
                return;
            }
            for (chess_move, tree) in self.moves.iter() {
                for _ in 0..depth {
                    print!("  |");
                }
                print!("-");
                println!("{}", chess_move.name(&self.board_state).unwrap());
                tree.print_tree(depth + 1, max_depth);
            }
        }
    }

    #[derive(Debug)]
    pub struct Engine {
        moves: BinaryHeap<ValuedMoveLocation>,
        move_tree: MoveTree,
        static_eval_weights: StaticEvaluationWeights,
        interest_eval_weights: InterestEvaluationWeights,
        minimax_settings: MinimaxSettings,
    }

    #[derive(Debug, Clone)]
    pub struct StaticEvaluationWeights {
        pub square_control_weight: f32,
        pub check_weight: f32,
        pub value_weight: f32,
        pub depth_cost: NotNan<f32>,
        pub past_pawn_weight: f32,
    }

    #[derive(Debug, Clone)]
    pub struct MinimaxSettings {
        pub min_depth: i32,
    }

    impl MinimaxSettings {
        pub fn new() -> MinimaxSettings {
            MinimaxSettings { min_depth: 2 }
        }
    }

    #[derive(Debug, Clone)]
    pub struct InterestEvaluationWeights {
        pub square_control_weight: f32,
        pub capture_weight: f32,
        pub home_row_pawn_weight: f32,
        pub check_weight: f32,
        pub king_moving_bonus: f32,
        pub queen_moving_bonus: f32,
        pub rook_moving_bonus: f32,
        pub minor_piece_moving_bouns: f32,
        pub attack_weight: f32,
    }

    impl InterestEvaluationWeights {
        pub fn new() -> Self {
            InterestEvaluationWeights {
                square_control_weight: 0.2,
                capture_weight: 2.5,
                home_row_pawn_weight: 3.5,
                check_weight: 10.0,
                king_moving_bonus: -2.0,
                queen_moving_bonus: 1.50,
                rook_moving_bonus: 3.0,
                minor_piece_moving_bouns: 7.0,
                attack_weight: 0.75,
            }
        }
    }

    impl StaticEvaluationWeights {
        pub fn new() -> StaticEvaluationWeights {
            StaticEvaluationWeights {
                square_control_weight: 0.05,
                check_weight: 3.0,
                value_weight: 1.0,
                depth_cost: NotNan::new(15.0).unwrap(),
                past_pawn_weight: 0.5,
            }
        }
    }

    #[derive(Clone, Debug)]
    pub enum EngineError {
        InvalidLocationError,
        NoValidMovesErrror,
        IllegalMoveError,
    }

    impl<'a> Engine {
        pub fn new(
            board: Board,
            static_eval_weights: StaticEvaluationWeights,
            interest_eval_weights: InterestEvaluationWeights,
            minimax_settings: MinimaxSettings,
        ) -> Engine {
            let mut res = Engine {
                moves: BinaryHeap::new(),
                move_tree: MoveTree {
                    moves: BTreeMap::new(),
                    board_state: board,
                },
                interest_eval_weights,
                static_eval_weights,
                minimax_settings,
            };
            res.generate_all_moves(VecDeque::new()).unwrap();
            res
        }

        pub fn get_board(&'a self) -> &'a Board {
            &self.move_tree.board_state
        }

        //go to a branch specified by the list of moves in location.
        fn go_to_location(
            &'a mut self,
            location: &VecDeque<ChessMove>,
        ) -> Result<&'a mut MoveTree, EngineError> {
            let mut current_tree = &mut self.move_tree;
            for chess_move in location {
                match current_tree.moves.get_mut(chess_move) {
                    Some(child_tree) => {
                        current_tree = child_tree;
                    }
                    None => {
                        return Err(EngineError::InvalidLocationError);
                    }
                }
            }
            Ok(current_tree)
        }

        pub fn interpret_and_make_move(&mut self, move_string: &str) -> Result<(), EngineError> {
            let chess_move = self
                .move_tree
                .board_state
                .interpret_move(move_string)
                .ok()
                .ok_or(EngineError::IllegalMoveError)?;
            self.make_move(&chess_move)?;
            Ok(())
        }

        pub fn make_move(&mut self, chess_move: &ChessMove) -> Result<(), EngineError> {
            self.move_tree = self
                .move_tree
                .moves
                .get_mut(chess_move)
                .ok_or(EngineError::IllegalMoveError)?
                .clone();
            self.moves
                .retain(|x| x.location.len() > 0 && x.location[0] == *chess_move);

            let mut new_moves = self.moves.clone().into_vec();

            new_moves.iter_mut().for_each(|x| {
                x.location.pop_front();
            });

            self.moves = BinaryHeap::from(new_moves);

            if self.move_tree.moves.is_empty() {
                self.generate_all_moves(VecDeque::new()).unwrap();
            }

            Ok(())
        }

        fn generate_all_moves(&mut self, location: VecDeque<ChessMove>) -> Result<(), EngineError> {
            let interest_weights = self.interest_eval_weights.clone();
            let depth_cost = self.static_eval_weights.depth_cost.clone();

            let tree_mut = self.go_to_location(&location)?;
            for i in 0..BOARD_SIZE {
                for j in 0..BOARD_SIZE {
                    match tree_mut.board_state.generate_moves(i, j) {
                        Ok(moves) => {
                            for chess_move in moves {
                                let mut new_board = tree_mut.board_state.clone();
                                match new_board.make_legal_move(chess_move) {
                                    Ok(()) => {
                                        tree_mut.moves.insert(
                                            chess_move,
                                            MoveTree {
                                                board_state: new_board,
                                                moves: BTreeMap::new(),
                                            },
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

            for (chess_move, ending_board) in tree_mut.moves.iter() {
                new_moves.push(ValuedMoveLocation {
                    valued_move: ValuedChessMove {
                        chess_move: chess_move.clone(),
                        value: Engine::evaluate_interest(
                            &interest_weights,
                            chess_move,
                            &tree_mut.board_state,
                            &ending_board.board_state,
                        )
                        .unwrap(),
                    },
                    location: location.clone(),
                    depth_cost,
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
            location.push_back(next_move.valued_move.chess_move);
            self.generate_all_moves(location)?;
            Ok(())
        }

        pub fn multi_thread_think_next_num_moves(self, thread_count: usize, num: usize) -> Engine {
            let engine_arc: Arc<Mutex<Engine>> = Arc::new(Mutex::new(self));
            let mut threads = Vec::new();
            let counter = Arc::new(Mutex::new(0));
            for _ in 0..thread_count {
                let my_engine = engine_arc.clone();
                let my_counter = counter.clone();
                let my_num = num.clone();
                threads.push(thread::spawn(move || {
                    loop {
                        match Engine::think_next_move_cocurrent(&*my_engine) {
                            Ok(_) => {}
                            Err(EngineError::NoValidMovesErrror) => break,
                            Err(err) => {
                                Result::<(), EngineError>::Err(err).unwrap();
                            }
                        };
                        let mut counter_lock = my_counter.lock().unwrap();
                        *counter_lock += 1;
                        if *counter_lock > my_num {
                            break;
                        }
                    }

                    return;
                }))
            }

            for thread in threads {
                thread.join().unwrap();
            }

            let lock = Arc::try_unwrap(engine_arc).expect("Lock still has multiple owners");
            lock.into_inner().expect("Mutex cannot be locked")
        }

        pub fn multi_thread_think_next_moves_until_stop(
            self,
            thread_count: usize,
            stopper: Receiver<()>,
        ) -> (Engine, usize) {
            let keep_going = Arc::new(Mutex::new(true));
            let engine_arc: Arc<Mutex<Engine>> = Arc::new(Mutex::new(self));
            let mut threads = Vec::new();
            let counter = Arc::new(Mutex::new(0));
            for _ in 0..thread_count {
                let my_engine = engine_arc.clone();
                let my_keep_going = keep_going.clone();
                let my_counter = counter.clone();
                threads.push(thread::spawn(move || {
                    loop {
                        Engine::think_next_move_cocurrent(&*my_engine).unwrap();
                        let keep_going_lock = my_keep_going.lock().unwrap();
                        let mut counter_lock = my_counter.lock().unwrap();
                        *counter_lock += 1;
                        if !*keep_going_lock {
                            break;
                        }
                    }
                    return;
                }))
            }

            stopper.recv().unwrap();

            let mut keep_going_lock = keep_going.lock().unwrap();

            *keep_going_lock = false;

            drop(keep_going_lock);

            for thread in threads {
                thread.join().unwrap();
            }

            let engine_lock = Arc::try_unwrap(engine_arc).expect("Lock still has multiple owners");
            let counter_lock = Arc::try_unwrap(counter).expect("Lock still has multiple owners");
            (
                engine_lock.into_inner().expect("Mutex cannot be locked"),
                counter_lock.into_inner().expect("Mutex cannot be locked"),
            )
        }

        pub fn think_next_move_cocurrent(engine: &Mutex<Engine>) -> Result<(), EngineError> {
            let mut engine_access = engine.lock().unwrap();
            let next_move = engine_access
                .moves
                .pop()
                .ok_or(EngineError::NoValidMovesErrror)?;
            drop(engine_access);
            let mut location = next_move.location;
            location.push_back(next_move.valued_move.chess_move);
            Engine::generate_all_moves_cocurrent(engine, location)
        }

        fn generate_all_moves_cocurrent(
            engine: &Mutex<Engine>,
            location: VecDeque<ChessMove>,
        ) -> Result<(), EngineError> {
            //get weights
            let mut engine_access = engine.lock().unwrap();

            let interest_weights = engine_access.interest_eval_weights.clone();
            let depth_cost = engine_access.static_eval_weights.depth_cost.clone();

            //copy board to work on local thread
            let tree = engine_access.go_to_location(&location)?.clone();
            //free engine for others to use
            drop(engine_access);
            //generate moves
            let mut all_moves = Vec::<(ChessMove, Board)>::new();

            for i in 0..BOARD_SIZE {
                for j in 0..BOARD_SIZE {
                    match tree.board_state.generate_moves(i, j) {
                        Ok(moves) => {
                            for chess_move in moves {
                                let mut new_board = tree.board_state.clone();
                                match new_board.make_legal_move(chess_move) {
                                    Ok(()) => {
                                        all_moves.push((chess_move, new_board));
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

            //store all the moves in proper formats
            let valued_move_locations: Vec<ValuedMoveLocation> = all_moves
                .iter()
                .map(|x: &(ChessMove, Board)| ValuedMoveLocation {
                    valued_move: ValuedChessMove {
                        value: Engine::evaluate_interest(
                            &interest_weights,
                            &x.0,
                            &tree.board_state,
                            &x.1,
                        )
                        .unwrap(),
                        chess_move: x.0.clone(),
                    },
                    location: location.clone(),
                    depth_cost,
                })
                .collect();

            let mut move_map = BTreeMap::new();

            for (chesss_move, ending_board) in all_moves {
                move_map.insert(
                    chesss_move,
                    MoveTree {
                        board_state: ending_board,
                        moves: BTreeMap::new(),
                    },
                );
            }

            //get back on engine to add values
            engine_access = engine.lock().unwrap();

            let real_tree = engine_access.go_to_location(&location)?;

            real_tree.moves = move_map;

            for valued_move_location in valued_move_locations {
                engine_access.moves.push(valued_move_location);
            }

            drop(engine_access);
            //yipee!

            Ok(())
        }

        fn evaluate_interest(
            interest_eval_weights: &InterestEvaluationWeights,
            chess_move: &ChessMove,
            starting_board: &Board,
            ending_board: &Board,
        ) -> Result<NotNan<f32>, BoardError> {
            use ChessMove::*;
            //temporary
            let res = match chess_move {
                Normal(normal_move) => Engine::evaluate_normal_move_interest(
                    &interest_eval_weights,
                    normal_move,
                    starting_board,
                    ending_board,
                )?,
                Promotion(normal_move, piece_kind) => {
                    Engine::evaluate_normal_move_interest(
                        &interest_eval_weights,
                        normal_move,
                        starting_board,
                        ending_board,
                    )? + piece_worth_king_inf(*piece_kind)
                }
                Castling(_) => NotNan::new(20.0).unwrap(),
            };

            Ok(res)
        }

        fn evaluate_normal_move_interest(
            interest_eval_weights: &InterestEvaluationWeights,
            normal_move: &NormalChessMove,
            starting_board: &Board,
            ending_board: &Board,
        ) -> Result<NotNan<f32>, BoardError> {
            Ok(
                match starting_board.get_piece(normal_move.initial_row, normal_move.initial_col)? {
                    None => return Err(BoardError::NoPieceError),
                    Some(Piece {
                        kind: PieceKind::Pawn,
                        color,
                    }) => {
                        let past = is_past_pawn(
                            normal_move.destination_row,
                            normal_move.destination_col,
                            ending_board,
                            color,
                        );
                        let value = match past {
                            true => NotNan::new(normal_move.destination_row as f32).unwrap(),
                            false => NotNan::new(0.0).unwrap(),
                        } + match normal_move.initial_row
                            == match color {
                                Color::Black => 6,
                                Color::White => 1,
                            } {
                            true => interest_eval_weights.home_row_pawn_weight,
                            false => 0.0,
                        };
                        value
                    }
                    Some(Piece {
                        kind: PieceKind::Bishop | PieceKind::Knight,
                        color: _,
                    }) => NotNan::new(interest_eval_weights.minor_piece_moving_bouns).unwrap(),
                    Some(Piece {
                        kind: PieceKind::Rook,
                        color: _,
                    }) => NotNan::new(interest_eval_weights.rook_moving_bonus).unwrap(),
                    Some(Piece {
                        kind: PieceKind::Queen,
                        color: _,
                    }) => NotNan::new(interest_eval_weights.queen_moving_bonus).unwrap(),
                    Some(Piece {
                        kind: PieceKind::King,
                        color: _,
                    }) => NotNan::new(interest_eval_weights.king_moving_bonus).unwrap(),
                } + match starting_board
                    .get_piece(normal_move.destination_row, normal_move.destination_col)?
                {
                    Some(piece) => piece_worth_king_inf(piece.kind),
                    None => NotNan::new(0.0).unwrap(),
                } + match ending_board.is_check.is_some() {
                    true => match ending_board.is_checkmate.is_some() {
                        true => INFINITY,
                        false => interest_eval_weights.check_weight,
                    },
                    false => 0.0,
                } + match starting_board
                    .get_piece(normal_move.destination_row, normal_move.destination_col)
                {
                    Ok(Some(piece)) => piece_worth_king_inf(piece.kind),
                    Ok(None) => NotNan::new(0.0).unwrap(),
                    Err(_) => panic!(),
                } * interest_eval_weights.capture_weight
                    + ((Engine::controlling_squares(ending_board, starting_board.get_turn())
                        - Engine::controlling_squares(starting_board, starting_board.get_turn()))
                        as f32)
                        * interest_eval_weights.square_control_weight
                    + Engine::evaluate_total_attack(ending_board)
                        * interest_eval_weights.attack_weight,
            )
        }

        fn evaluate_total_attack(board: &Board) -> NotNan<f32> {
            let mut sum = NotNan::new(0.0).unwrap();
            for i in 0..BOARD_SIZE {
                for j in 0..BOARD_SIZE {
                    let piece = board.get_piece(i, j).unwrap();
                    match piece {
                        None => continue,
                        Some(piece) => {
                            if piece.color == board.get_turn() {
                                continue;
                            }
                        }
                    }
                    let moves = match board.generate_moves_ignore_turn(i, j) {
                        Ok(chess_moves) => chess_moves,
                        Err(BoardError::NoPieceError | BoardError::WrongTurnError) => continue,
                        Err(_) => panic!(),
                    };
                    for chess_move in moves {
                        match chess_move {
                            ChessMove::Normal(normal_move) => {
                                let attacked_piece = match board.get_piece(
                                    normal_move.destination_row,
                                    normal_move.destination_col,
                                ) {
                                    Ok(Some(piece)) => piece,
                                    Ok(None) => continue,
                                    Err(_) => panic!(),
                                };
                                sum += piece_worth_king_zero(attacked_piece.kind);
                            }
                            ChessMove::Castling(_) => {
                                continue;
                            }
                            ChessMove::Promotion(normal_move, _) => {
                                let attacked_piece = match board.get_piece(
                                    normal_move.destination_row,
                                    normal_move.destination_col,
                                ) {
                                    Ok(Some(piece)) => piece,
                                    Ok(None) => continue,
                                    Err(_) => panic!(),
                                };
                                sum += piece_worth_king_zero(attacked_piece.kind);
                            }
                        }
                    }
                }
            }
            sum
        }

        pub fn eval_and_best_move(&self) -> (Eval, Option<ChessMove>) {
            Engine::minimax(
                &self,
                &self.move_tree,
                0,
                self.minimax_settings.min_depth,
                1000,
                self.move_tree.board_state.get_turn() == Color::White,
            )
        }

        pub fn parallel_eval_and_best_move(
            self: Arc<Self>,
            thread_count: usize,
        ) -> (Eval, Option<ChessMove>) {
            let thread_count_arc = Arc::new(Mutex::new(thread_count - 1));
            self.clone().parallel_minimax(
                &self.move_tree,
                0,
                self.minimax_settings.min_depth,
                1000,
                self.move_tree.board_state.get_turn() == Color::White,
                thread_count_arc,
            )
        }

        fn minimax(
            &self,
            tree: &MoveTree,
            depth: i32,
            min_depth: i32,
            max_depth: i32,
            maximizing_player: bool,
        ) -> (Eval, Option<ChessMove>) {
            if depth == max_depth || tree.is_leaf() {
                let eval = self.static_evaluation(&tree.board_state);
                if let Eval::Numeric(_) = eval {
                    if depth < min_depth {
                        return (
                            Eval::Numeric(
                                NotNan::new(INFINITY).unwrap()
                                    * match maximizing_player {
                                        true => 1.0,
                                        false => -1.0,
                                    },
                            ),
                            None,
                        );
                    }
                }
                return (eval, None);
            }

            if maximizing_player {
                let mut max_eval = Eval::MateIn(Color::Black, -1);
                let mut best_move = None;
                for (chess_move, child) in tree.moves.clone() {
                    let eval = self
                        .minimax(&child, depth + 1, min_depth, max_depth, false)
                        .0
                        .increase_mate_counter();
                    if eval > max_eval {
                        max_eval = eval;
                        best_move = Some(chess_move)
                    }
                }
                return (max_eval, best_move);
            } else {
                let mut min_eval = Eval::MateIn(Color::White, -1);
                let mut best_move = None;
                for (chess_move, child) in tree.moves.clone() {
                    let eval = self
                        .minimax(&child, depth + 1, min_depth, max_depth, true)
                        .0
                        .increase_mate_counter();
                    if eval < min_eval {
                        min_eval = eval;
                        best_move = Some(chess_move)
                    }
                }
                return (min_eval, best_move);
            }
        }

        fn parallel_minimax(
            self: Arc<Self>,
            tree: &MoveTree,
            depth: i32,
            min_depth: i32,
            max_depth: i32,
            maximizing_player: bool,
            threads_left_arc: Arc<Mutex<usize>>,
        ) -> (Eval, Option<ChessMove>) {
            if depth == max_depth || tree.is_leaf() {
                return self.minimax(tree, depth, min_depth, max_depth, maximizing_player);
            }

            let prefer_eval_predicate = match maximizing_player {
                true => |x: &Eval, y: &Eval| x > y,
                false => |x: &Eval, y: &Eval| x < y,
            };

            let mut best_eval = Eval::MateIn(
                match maximizing_player {
                    true => Color::Black,
                    false => Color::White,
                },
                -1,
            );
            let mut best_move = None;
            let moves = tree.moves.clone();
            let mut threads = Vec::new();

            let mut sub_evals = Vec::<(Eval, ChessMove)>::new();

            for (chess_move, child) in moves {
                let my_self = self.clone();
                let my_threads_left_arc = threads_left_arc.clone();
                let mut threads_left = threads_left_arc.lock().unwrap();
                if *threads_left >= 1 {
                    let thread = thread::spawn(move || {
                        let res = my_self.parallel_minimax(
                            &child,
                            depth + 1,
                            min_depth,
                            max_depth,
                            !maximizing_player,
                            my_threads_left_arc,
                        );
                        (res.0.increase_mate_counter(), chess_move)
                    });
                    *threads_left -= 1;
                    drop(threads_left);
                    threads.push(thread);
                } else {
                    drop(threads_left);
                    let res = my_self.parallel_minimax(
                        &child,
                        depth + 1,
                        min_depth,
                        max_depth,
                        !maximizing_player,
                        my_threads_left_arc,
                    );
                    sub_evals.push((res.0.increase_mate_counter(), chess_move));
                }
            }

            for thread in threads {
                sub_evals.push(thread.join().unwrap());
            }

            for (eval, chess_move) in sub_evals {
                if prefer_eval_predicate(&eval, &best_eval) {
                    best_eval = eval;
                    best_move = Some(chess_move);
                }
            }

            return (best_eval, best_move);
        }

        fn controlling_squares(board: &Board, color: Color) -> i32 {
            let mut squares = [[false; BOARD_SIZE]; BOARD_SIZE];
            for i in 0..BOARD_SIZE {
                for j in 0..BOARD_SIZE {
                    match board.get_piece(i, j).expect("Cant error always in bounds") {
                        Some(piece) => {
                            if piece.color != color {
                                continue;
                            }
                            for chess_move in board
                                .generate_moves_ignore_turn(i, j)
                                .expect("we know there's a piece there")
                            {
                                match chess_move {
                                    ChessMove::Normal(normal_move) => {
                                        squares[normal_move.destination_row]
                                            [normal_move.destination_col] = true;
                                    }
                                    ChessMove::Castling(_) => {}
                                    ChessMove::Promotion(normal_move, _) => {
                                        squares[normal_move.destination_row]
                                            [normal_move.destination_col] = true;
                                    }
                                }
                            }
                        }
                        None => {}
                    }
                }
            }
            let count = squares.iter().flatten().filter(|&&x| x).count();
            count as i32
        }

        fn static_evaluation(&self, board_state: &Board) -> Eval {
            match board_state.is_checkmate {
                None => {}
                Some(GameEnd::Mated(Color::White)) => return Eval::MateIn(Color::Black, 0),
                Some(GameEnd::Mated(Color::Black)) => return Eval::MateIn(Color::White, 0),
                Some(GameEnd::StaleMate) => return Eval::Numeric(NotNan::new(0.0).unwrap()),
            };
            let mut res = NotNan::new(0.0).unwrap();
            res += (Engine::controlling_squares(board_state, Color::White) as f32)
                * self.static_eval_weights.square_control_weight;
            res -= (Engine::controlling_squares(board_state, Color::Black) as f32)
                * self.static_eval_weights.square_control_weight;
            res += match board_state.is_check {
                Some(Color::Black) => self.static_eval_weights.check_weight,
                None => 0.0,
                Some(Color::White) => -self.static_eval_weights.check_weight,
            };
            for i in 0..BOARD_SIZE {
                for j in 0..BOARD_SIZE {
                    let piece_option = board_state.get_piece(i, j).unwrap();
                    res += match piece_option {
                        Some(piece) => match piece.kind {
                            PieceKind::Pawn => {
                                piece_worth_king_zero(piece.kind)
                                    * match piece.color {
                                        Color::White => 1.0,
                                        Color::Black => -1.0,
                                    }
                                    + match is_past_pawn(i, j, board_state, piece.color) {
                                        true => match piece.color {
                                            Color::Black => 8.0 - (i as f32),
                                            Color::White => i as f32,
                                        },
                                        false => 0.0,
                                    } * match piece.color {
                                        Color::White => 1.0,
                                        Color::Black => -1.0,
                                    } * self.static_eval_weights.past_pawn_weight
                            }
                            _ => {
                                piece_worth_king_zero(piece.kind)
                                    * match piece.color {
                                        Color::White => 1.0,
                                        Color::Black => -1.0,
                                    }
                                    * self.static_eval_weights.value_weight
                            }
                        },
                        None => NotNan::new(0.0).unwrap(),
                    }
                }
            }
            Eval::Numeric(res)
        }
    }

    fn is_past_pawn(row: usize, col: usize, board: &Board, color: Color) -> bool {
        let to_left_option = col.checked_sub(1);
        let to_center = col;
        let to_right = col + 1;

        let cols = match to_left_option {
            Some(to_left) => vec![to_left, to_center, to_right],
            None => vec![to_center, to_right],
        };

        let mut past = true;
        for i in match color {
            Color::White => row..BOARD_SIZE,
            Color::Black => 0..row,
        } {
            for j in cols.clone() {
                let piece_result = board.get_piece(i, j);
                match piece_result {
                    Ok(Some(piece)) => {
                        if piece
                            == (Piece {
                                kind: PieceKind::Pawn,
                                color: color.opposite(),
                            })
                        {
                            past = false;
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
        past
    }
}
