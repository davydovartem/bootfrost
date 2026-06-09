use crate::term::*;
use crate::answer::*;
use crate::misc::*;
use crate::solver::*;
use crate::strategies::environment::*;
use crate::strategies::attributes::*;
use crate::strategies::strategies::Strategy;
use crate::strategies::rhai_runtime::{self, RhaiRuntime, RhaiRuntimeError};
use rhai::Dynamic;
use std::fs;
use std::collections::HashMap;

type IFunction = fn(&Vec<TermId>, &mut PEnv) -> TermId;

fn plus(a:i64,b:i64) -> i64 {a + b}
fn minus(a:i64,b:i64) -> i64 {a - b}
fn multiply(a:i64,b:i64) -> i64 {a * b}
fn divide(a:i64,b:i64) -> i64 {a / b}

// fn eq(a:i64, b:i64) -> bool {a == b}
// fn noteq(a:i64, b:i64) -> bool {a != b}
fn lt(a:i64, b:i64) -> bool {a < b}
fn gt(a:i64, b:i64) -> bool {a > b}
fn lteq(a:i64, b:i64) -> bool {a <= b}
fn gteq(a:i64, b:i64) -> bool {a >= b}

fn lt_float(a:f64, b:f64) -> bool {a < b}
fn gt_float(a:f64, b:f64) -> bool {a > b}
fn lteq_float(a:f64, b:f64) -> bool {a <= b}
fn gteq_float(a:f64, b:f64) -> bool {a >= b}


macro_rules! result_term{
	($res:expr, bool) => {
		Term::Bool($res)
	};
	($res:expr, i64) => {
		Term::Integer($res)
	};
	($res:expr, f64) => {
		Term::Float($res)
	}
}

macro_rules! ifunction_binary_integers{
	($f:tt, $tp:tt) => {
		|args: &Vec<TermId>, env: &mut PEnv| -> TermId{
			if args.len() != 2{
				panic!("");
			}

			let arg0 = env.psterms.get_term(&args[0]);
			let arg1 = env.psterms.get_term(&args[1]);

			let (n1,n2) = if let (Term::Integer(_n1), Term::Integer(_n2)) = (arg0, arg1){
				(_n1, _n2)
			}else{
				panic!("");
			};

			env.psterms.get_tid(result_term!($f(n1,n2), $tp)).unwrap()
		}
	}
}

macro_rules! ifunction_binary_floats{
	($f:tt, $tp:tt) => {
		|args: &Vec<TermId>, env: &mut PEnv| -> TermId{
			if args.len() != 2{
				panic!("");
			}

			let arg0 = env.psterms.get_term(&args[0]);
			let arg1 = env.psterms.get_term(&args[1]);

			let (n1,n2) = match (arg0, arg1) {
				(Term::Float(_n1), Term::Float(_n2)) => {
					(_n1, _n2)
				},
				(Term::Integer(_n1), Term::Integer(_n2)) => {
					(_n1 as f64, _n2 as f64)
				},
				(Term::Integer(_n1), Term::Float(_n2)) => {
					(_n1 as f64, _n2)
				},
				(Term::Float(_n1), Term::Integer(_n2)) => {
					(_n1, _n2 as f64)
				},
				_ => {
					panic!("");
				}
			};

			env.psterms.get_tid(result_term!($f(n1,n2), $tp)).unwrap()
		}
	}
}

// lists
fn push1(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 2{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);
	let arg1 = env.psterms.get_term(&args[1]);

	let mut list = if let Term::List(_n1) = arg0{
		_n1.clone()
	}else{
		panic!("");
	};

	list.push(args[1]);
	env.psterms.get_tid(Term::List(list)).unwrap()
}

// lists
fn last1(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 1{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);

	let list = if let Term::List(_n1) = arg0{
		_n1
	}else{
		panic!("");
	};

	let res = list.last().unwrap();
	return *res
}

// lists
fn first1(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 1{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);

	let list = if let Term::List(_n1) = arg0{
		_n1
	}else{
		panic!("");
	};

	let res = list.first().unwrap();
	return *res
}

//lists
fn notempty(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 1{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);

	let list = if let Term::List(_n1) = arg0{
		_n1
	}else{
		panic!("");
	};

	let res = !list.is_empty();

	env.psterms.get_tid(Term::Bool(res)).unwrap()
}

//lists
fn subseteq(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 2{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);
	let arg1 = env.psterms.get_term(&args[1]);

	let list = if let Term::List(_n1) = arg1{
		_n1
	}else{
		panic!("");
	};

	let res = if let Term::List(_n2) = arg0{
		_n2.iter().all(|x|list.contains(&x))
	}else{
		panic!("");
	};

	env.psterms.get_tid(Term::Bool(res)).unwrap()
}


//lists
fn inlist(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 2{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);
	let arg1 = env.psterms.get_term(&args[1]);

	let list = if let Term::List(_n1) = arg1{
		_n1
	}else{
		panic!("");
	};

	// let res = if let Term::List(_n2) = arg0{
	// 	_n2.iter().all(|x|list.contains(&x))
	// }else{
	// 	list.contains(&args[0])		
	// };

	let res = list.contains(&args[0]);

	env.psterms.get_tid(Term::Bool(res)).unwrap()
}

//lists
fn notinlist(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 2{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);
	let arg1 = env.psterms.get_term(&args[1]);

	let list = if let Term::List(_n1) = arg1{
		_n1
	}else{
		panic!("");
	};

	let res = !list.contains(&args[0]);

	env.psterms.get_tid(Term::Bool(res)).unwrap()
}


// lists
fn sortlist(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 1{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);

	let mut list = if let Term::List(_n1) = arg0{
		_n1.clone()
	}else{
		panic!("");
	};

	let res = if list.iter().all(|x| env.psterms.is_integer(x)){
		let mut list_i: Vec<i64> = list.iter().map(|x|
			if let Term::Integer(i) = env.psterms.get_term(x){
				i
			}else{
				panic!("")
			}
		).collect();

		list_i.sort();
		list_i.iter().map(|x|env.psterms.get_tid(Term::Integer(*x)).unwrap()).collect::<Vec<TermId>>()
	}else{
		panic!("");
	};

	// list.push(args[1]);
	env.psterms.get_tid(Term::List(res)).unwrap()
}


// lists
fn dedup(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 1{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);

	let list = if let Term::List(_n1) = arg0{
		_n1
	}else{
		panic!("");
	};

	// let res = if list.iter().all(|x| env.psterms.is_integer(x)){
	// 	let mut list_i: Vec<i64> = list.iter().map(|x|
	// 		if let Term::Integer(i) = env.psterms.get_term(x){
	// 			i
	// 		}else{
	// 			panic!("")
	// 		}
	// 	).collect();

	// 	// let list_i: Vec<i64> = list.iter().unique().collect()
	// 	// list_i.iter().map(|x|env.psterms.get_tid(Term::Integer(*x)).unwrap()).collect::<Vec<TermId>>()
	// }else{
	// 	panic!("");
	// };
	let mut res: Vec<TermId> = vec![];
	for x in list{
		if !res.contains(&x){
			res.push(x.clone());
		}
	}	

	// let res = res_i.iter().map(|x|env.psterms.get_tid(Term::Integer(*x)).unwrap()).collect::<Vec<TermId>>();

	// list.push(args[1]);
	env.psterms.get_tid(Term::List(res)).unwrap()
}

// lists
fn get_1d(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	let arg0 = env.psterms.get_term(&args[0]);
	let arg1 = env.psterms.get_term(&args[1]);

	let i = if let Term::Integer(i) = arg0{
		i
	}else{
		panic!("get(i, list): the first argument must be an integer index");
	};

	let list = if let Term::List(l) = arg1{
		l
	}else{
		panic!("get(i, list): the second argument must be a list");
	};

	if i < 0 || (i as usize) >= list.len(){
		panic!("get(i, list): index {} is out of bounds (list length {})", i, list.len());
	}

	list[i as usize]
}

// lists
fn get_at(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	match args.len(){
		2 => get_1d(args, env),
		3 => {
			// get(i, j, matrix) == matrix[i][j]  (i - row, j - column)
			let arg_i = env.psterms.get_term(&args[0]);
			let i = if let Term::Integer(i) = arg_i{
				i
			}else{
				panic!("get(i, j, matrix): the first argument must be an integer index of the row");
			};

			let matrix = env.psterms.get_term(&args[2]);
			let rows = if let Term::List(rows) = matrix{
				rows
			}else{
				panic!("get(i, j, matrix): the third argument must be a list of lists");
			};

			if i < 0 || (i as usize) >= rows.len(){
				panic!("get(i, j, matrix): index {} is out of bounds (rows count {})", i, rows.len());
			}

			let row_tid = rows[i as usize];
			let row_term = env.psterms.get_term(&row_tid);
			let row = if let Term::List(r) = row_term{
				r
			}else{
				panic!("get(i, j, matrix): row {} is not a list", i);
			};

			let j = if let Term::Integer(j) = env.psterms.get_term(&args[1]){
				j
			}else{
				panic!("get(i, j, matrix): the second argument must be an integer index of the column");
			};

			if j < 0 || (j as usize) >= row.len(){
				panic!("get(i, j, matrix): index {} is out of bounds (row length {})", j, row.len());
			}

			row[j as usize]
		},
		_ => panic!("get: expected 2 (get(i, list)) or 3 (get(i, j, matrix)) arguments, got {}", args.len()),
	}
}

// lists
fn set_1d(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	let arg0 = env.psterms.get_term(&args[0]);
	let arg2 = env.psterms.get_term(&args[2]);

	let i = if let Term::Integer(i) = arg0{
		i
	}else{
		panic!("set(i, val, list): the first argument must be an integer index");
	};

	let mut list = if let Term::List(l) = arg2{
		l.clone()
	}else{
		panic!("set(i, val, list): the third argument must be a list");
	};

	if i < 0 || (i as usize) >= list.len(){
		panic!("set(i, val, list): index {} is out of bounds (list length {})", i, list.len());
	}

	list[i as usize] = args[1];
	env.psterms.get_tid(Term::List(list)).unwrap()
}

// lists
fn set_at(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	match args.len(){
		3 => set_1d(args, env),
		4 => {
			// set(i, j, val, matrix): matrix[i][j] = val  (i - row, j - column)
			let arg_i = env.psterms.get_term(&args[0]);
			let arg_j = env.psterms.get_term(&args[1]);

			let i = if let Term::Integer(i) = arg_i{
				i
			}else{
				panic!("set(i, j, val, matrix): the first argument must be an integer index of the row");
			};
			let j = if let Term::Integer(j) = arg_j{
				j
			}else{
				panic!("set(i, j, val, matrix): the second argument must be an integer index of the column");
			};

			let matrix = env.psterms.get_term(&args[3]);
			let mut rows = if let Term::List(rows) = matrix{
				rows.clone()
			}else{
				panic!("set(i, j, val, matrix): the third argument must be a list of lists");
			};

			if i < 0 || (i as usize) >= rows.len(){
				panic!("set(i, j, val, matrix): index {} is out of bounds (rows count {})", i, rows.len());
			}

			let row_tid = rows[i as usize];
			let mut row = if let Term::List(r) = env.psterms.get_term(&row_tid){
				r.clone()
			}else{
				panic!("set(i, j, val, matrix): row i={} is not a list", i);
			};

			if j < 0 || (j as usize) >= row.len(){
				panic!("set(i, j, val, matrix): column index j={} is out of bounds (row {} length: {})", j, i, row.len());
			}

			row[j as usize] = args[2];
			rows[i as usize] = env.psterms.get_tid(Term::List(row)).unwrap();
			env.psterms.get_tid(Term::List(rows)).unwrap()
		},
		_ => panic!("set: expected 3 (set(i, val, list)) or 4 (set(i, j, val, matrix)) arguments, got {}", args.len()),
	}
}


// string, lists
fn concat(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 2{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);
	let arg1 = env.psterms.get_term(&args[1]);

	match (arg0, arg1){
		(Term::String(n1), Term::String(n2)) => {
			let res = format!("{}{}",n1,n2);
			env.psterms.get_tid(Term::String(res)).unwrap()
		},
		(Term::List(n1), Term::List(n2)) => {
			let mut res = vec![];
			res.append(&mut n1.clone());
			res.append(&mut n2.clone());
			env.psterms.get_tid(Term::List(res)).unwrap()
		}
		_ => {
			panic!("");
		}
	}

}

fn replace(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 3{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);
	let arg1 = env.psterms.get_term(&args[1]);
	let arg2 = env.psterms.get_term(&args[2]);

	let (n1,n2,n3) = if let (Term::String(_n1), Term::String(_n2), Term::String(_n3)) = (arg0, arg1, arg2){
		(_n1, _n2, _n3)
	}else{
		panic!("");
	};
	
	let res = str::replace(&n1, &n2,&n3);
	env.psterms.get_tid(Term::String(res)).unwrap()
}

fn blen(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 0{
		panic!("");
	}	
	env.psterms.get_tid(Term::Integer(env.base.len().try_into().unwrap())).unwrap()	
}

fn base_to_string(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 0{
		panic!("");
	}	
	
	let bstr = env.base.base
		.iter()
		.filter(|x|!env.attributes.check(KeyObject::BaseAtom(x.id), AttributeName("deleted".to_string()), AttributeValue("true".to_string())))
		.map(|x|
			TidDisplay{
				tid: x.term,
				psterms: &env.psterms,
				context: None,
				dm: DisplayMode::Plain,
			}.to_string()).collect::<Vec<String>>().join(",");
	env.psterms.get_tid(Term::String(bstr)).unwrap()	
}


fn remove_fact(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 1{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);

	let i = if let Term::Integer(i) = arg0{
		i
	}else{
		panic!("");
	};

	if i < 0 || (i as usize) >= env.answer.log.len(){
		// print all log items to show the structure
		let mut log_dump = String::new();
		for (idx, item) in env.answer.log.iter().enumerate(){
			match item{
				LogItem::Matching{batom_id, qatom_i, ..} => {
					log_dump.push_str(&format!("  log[{}] = Matching (qatom_i={}, batom_id={})\n", idx, qatom_i, batom_id.0));
				},
				LogItem::Interpretation{qatom_i} => {
					log_dump.push_str(&format!("  log[{}] = Interpretation (qatom_i={})\n", idx, qatom_i));
				},
			}
		}
		panic!(
			"remove_fact: index {} is out of bounds (length of env.answer.log = {}).\n\
			 Content of env.answer.log:\n{}",
			i, env.answer.log.len(), log_dump
		);
	}

	let log_item = &env.answer.log[i as usize];

	let b = if let LogItem::Matching{batom_id: b, ..} = log_item{
		*b
	}else{
		match log_item{
			LogItem::Matching{..} => unreachable!(),
			LogItem::Interpretation{qatom_i} => {
				panic!(
					"remove_fact: on index {} in env.answer.log is Interpretation (qatom_i={}), \n
					 but not Matching. remove_fact can only delete Matching-atoms (corresponding to the base), \n
					 but not IFunctor (e.g., get/set/dist).\n",
					i, qatom_i
				);
			},
		}
	};

	env.attributes.set_attribute(KeyObject::BaseAtom(b), AttributeName("deleted".to_string()), AttributeValue("true".to_string()), env.bid);

	env.psterms.get_tid(Term::Bool(true)).unwrap()
}

fn answer_subquestion(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 1{
		panic!("answer_subquestion expects exactly one integer argument");
	}

	let arg0 = env.psterms.get_term(&args[0]);
	let i = if let Term::Integer(i) = arg0{
		i
	}else{
		panic!("answer_subquestion expects an integer argument");
	};

	if i < 0{
		panic!("answer_subquestion expects a non-negative integer");
	}

	env.answer_subquestions.push(i as usize);
	env.psterms.get_tid(Term::Bool(true)).unwrap()
}

fn answer_once(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if !args.is_empty(){
		panic!("answer_once expects no arguments");
	}

	env.answer_once = true;
	env.psterms.get_tid(Term::Bool(true)).unwrap()
}

pub fn print_batoms(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 0{
		panic!("");
	}

	print!("Terms used in the base: ");
	for b in env.answer.get_batoms().into_iter().flatten(){
		if let Some(bt) = env.base.get_by_id(b){
			print!("{}, ", TidDisplay{
				tid: bt.term,
				psterms: env.psterms,
				context: None,
				dm: DisplayMode::Plain,
			});
		}else{
			print!("<stale_batom_id:{}>, ", b.0);
		}
	}

	env.psterms.get_tid(Term::Bool(true)).unwrap()	
}

fn read_file_to_string(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 1{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);

	let n1 = if let Term::String(_n1) = arg0{
		_n1
	}else{
		panic!("");
	};
	
	let res = fs::read_to_string(&n1)
        .expect("Something went wrong reading the file");

	env.psterms.get_tid(Term::String(res)).unwrap()
}

fn solve(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 2{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);
	let arg1 = env.psterms.get_term(&args[1]);

	let n1 = if let Term::String(_n1) = arg0{
		_n1
	}else{
		panic!("");
	};
	
	let d = if let Term::Integer(_n2) = arg1{
		_n2
	}else{
		panic!("");
	};

	

	println!("\n");
    println!("*******************************************************");
    println!("**************** START OF SUBINFERENCE ****************");
    println!("*******************************************************");


    let mut solver = Solver::parse_string(&n1, Strategy::General);
    let res = solver.solver_loop(d.try_into().unwrap());
    let r = if SolverResultType::Refuted == res.t{
    	println!("**** REFUTED ****");
    	true
    }else{
    	println!("**** FAIL ****");
    	false
    };
    println!("*****************************************************");
    println!("**************** END OF SUBINFERENCE ****************");
    println!("*****************************************************");
    println!("\n");

	env.psterms.get_tid(Term::Bool(r)).unwrap()
}

fn string(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 1{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);

	let res = match arg0{
		Term::Integer(i) => i.to_string(),
		_ => "hello".to_string()
	};

	env.psterms.get_tid(Term::String(res)).unwrap()
}


fn noteq(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 2{
		panic!("");
	}
	let res = args[0] == args[1];

	env.psterms.get_tid(Term::Bool(!res)).unwrap()
}

fn eq(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 2{
		panic!("");
	}
	let res = args[0] == args[1];


	env.psterms.get_tid(Term::Bool(res)).unwrap()
}

fn or_bool(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 2{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);
	let arg1 = env.psterms.get_term(&args[1]);

	let (b0, b1) = if let (Term::Bool(_b0), Term::Bool(_b1)) = (arg0, arg1){
		(_b0, _b1)
	}else{
		panic!("");
	};

	env.psterms.get_tid(Term::Bool(b0 || b1)).unwrap()
}

fn and_bool(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 2{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);
	let arg1 = env.psterms.get_term(&args[1]);

	let (b0, b1) = if let (Term::Bool(_b0), Term::Bool(_b1)) = (arg0, arg1){
		(_b0, _b1)
	}else{
		panic!("");
	};

	env.psterms.get_tid(Term::Bool(b0 && b1)).unwrap()
}

fn bool2int(args: &Vec<TermId>, env: &mut PEnv) -> TermId{
	if args.len() != 1{
		panic!("");
	}

	let arg0 = env.psterms.get_term(&args[0]);

	let value = if let Term::Bool(flag) = arg0{
		if flag { 1 } else { 0 }
	}else{
		panic!("");
	};

	env.psterms.get_tid(Term::Integer(value)).unwrap()
}

fn dist(args: &Vec<TermId>, env: &mut PEnv) -> TermId {
    if args.len() != 4 {
        panic!("dist expects exactly 4 arguments: x0, y0, x1, y1");
    }

    let arg0 = env.psterms.get_term(&args[0]);
    let arg1 = env.psterms.get_term(&args[1]);
    let arg2 = env.psterms.get_term(&args[2]);
    let arg3 = env.psterms.get_term(&args[3]);

    let (x0, y0, x1, y1) = match (arg0, arg1, arg2, arg3) {
        (Term::Integer(x0), Term::Integer(y0), Term::Integer(x1), Term::Integer(y1)) => {
            (x0 as f64, y0 as f64, x1 as f64, y1 as f64)
        },
        (Term::Float(x0), Term::Float(y0), Term::Float(x1), Term::Float(y1)) => {
            (x0, y0, x1, y1)
        },
        (Term::Integer(x0), Term::Integer(y0), Term::Float(x1), Term::Float(y1)) => {
            (x0 as f64, y0 as f64, x1, y1)
        },
        (Term::Float(x0), Term::Float(y0), Term::Integer(x1), Term::Integer(y1)) => {
            (x0, y0, x1 as f64, y1 as f64)
        },
        (Term::Integer(x0), Term::Float(y0), Term::Integer(x1), Term::Float(y1)) => {
            (x0 as f64, y0, x1 as f64, y1)
        },
        (Term::Float(x0), Term::Integer(y0), Term::Float(x1), Term::Integer(y1)) => {
            (x0, y0 as f64, x1, y1 as f64)
        },
        (Term::Integer(x0), Term::Float(y0), Term::Float(x1), Term::Integer(y1)) => {
            (x0 as f64, y0, x1, y1 as f64)
        },
        (Term::Float(x0), Term::Integer(y0), Term::Integer(x1), Term::Float(y1)) => {
            (x0, y0 as f64, x1 as f64, y1)
        },
        _ => {
            panic!("dist arguments must be numbers (integers or floats)");
        }
    };

    let dx = x1 - x0;
    let dy = y1 - y0;
    let distance = (dx * dx + dy * dy).sqrt();

    env.psterms.get_tid(result_term!(distance, f64)).unwrap()
}

// ====
// Bridge to user-defined Rhai functions.
//
// `rhai_call(Name, Args)` is a single generic ifunction that lets a
// Bootfrost formula invoke *any* function exposed by a user-supplied
// Rhai script (loaded at startup via `--user-ifuncs`). The script
// receives a single array argument `args` and returns whatever value
// it likes. `rhai_call` returns a marshalled `Term::List` of the
// result elements (booleans, integers, nested arrays) so the formula
// can destructure with `first` / `last` or treat it as opaque.
//
// Adding a new user function does NOT require touching this file: just
// register the function in the `.rhai` script and reference its name
// from the formula.

fn rhai_call(args: &Vec<TermId>, env: &mut PEnv) -> TermId {
    if args.len() != 2 {
        panic!(
            "rhai_call expects exactly 2 arguments (name, args), got {}",
            args.len()
        );
    }

    let cache_key = (args[0], args[1]);
    if let Some(cached_tid) = env.rhai_call_cache.get(&cache_key) {
        return *cached_tid;
    }

    // First argument: function name as a Bootfrost string term.
    let name = match env.psterms.get_term(&args[0]) {
        Term::String(s) => s.clone(),
        other => panic!(
            "rhai_call: first argument must be a string (function name), got {:?}",
            other
        ),
    };

    // Second argument: list of values to forward to the Rhai function.
    // Marshalled into a single Rhai Array; the script destructures it.
    let params_dyn = match rhai_runtime::term_to_dynamic(args[1], env.psterms) {
        Ok(d) => d,
        Err(e) => panic!("rhai_call: failed to marshal args list: {}", e),
    };
    let params_array = match params_dyn.into_array() {
        Ok(a) => a,
        Err(e) => panic!("rhai_call: args must be a list, got {}", e),
    };

    // Invoke the Rhai function via the global slot. We always pass
    // exactly one argument (the array) to keep the call signature
    // independent of the user's arity.
    let result_dyn: rhai::Dynamic = match RhaiRuntime::with_global(|rt| {
        rt.call_fn(name.as_str(), (params_array,))
    }) {
        Ok(d) => d,
        Err(RhaiRuntimeError::ScriptNotLoaded) => {
            panic!(
                "rhai_call: no Rhai script has been installed; pass --user-ifuncs <file> on the command line"
            );
        }
        Err(e) => panic!("rhai_call: Rhai call failed: {}", e),
    };

    // Marshal the result back into a Term.
    //
    // PCSF's `==` on integer/boolean terms only succeeds when the two
    // operands share a `TermId` (terms are interned), so we MUST return
    // the scalar directly instead of wrapping it in a single-element
    // list. Wrapping would make
    //     rhai_call("a_star_path_exists", [...]) == 1
    // compare TermId(List) with TermId(Integer(1)) — always false.
    //
    // Arrays still come back as a `Term::List`, which the formula
    // destructures with `first` / `last`.
    let result_tid = if result_dyn.is_array() {
        let arr = result_dyn
            .into_array()
            .expect("just checked is_array");
        let mut items: Vec<TermId> = Vec::with_capacity(arr.len());
        for d in arr.into_iter() {
            items.push(
                rhai_runtime::dynamic_to_term(d, env.psterms)
                    .expect("rhai_call: failed to marshal result element"),
            );
        }
        env.psterms.get_tid(Term::List(items)).unwrap()
    } else {
        rhai_runtime::dynamic_to_term(result_dyn, env.psterms)
            .expect("rhai_call: failed to marshal scalar result")
    };

    env.rhai_call_cache.insert(cache_key, result_tid);
    result_tid
}

pub fn init() -> (PSTerms, HashMap<String, SymbolId>){
	let mut psterms = PSTerms::new();
	let mut fmap = HashMap::new();


	let fs = HashMap::from([
		("!=".to_string(), (noteq as IFunction, Position::Infix)),
		("==".to_string(), (eq as IFunction, Position::Infix)),
		("or".to_string(), (or_bool as IFunction, Position::Classic)),
		("and".to_string(), (and_bool as IFunction, Position::Classic)),
		("bool2int".to_string(), (bool2int as IFunction, Position::Classic)),
		("+".to_string(), (ifunction_binary_integers!(plus, i64) as IFunction, Position::Infix)),
		("-".to_string(), (ifunction_binary_integers!(minus, i64) as IFunction, Position::Infix)),
		("*".to_string(), (ifunction_binary_integers!(multiply, i64) as IFunction, Position::Infix)),
		("/".to_string(), (ifunction_binary_integers!(divide, i64) as IFunction, Position::Infix)),
		// ("==".to_string(), (ifunction_binary_integers!(eq, bool) as IFunction, Position::Infix)),
		// ("!=".to_string(), (ifunction_binary_integers!(noteq, bool) as IFunction, Position::Infix)),
		("<".to_string(), (ifunction_binary_integers!(lt, bool) as IFunction, Position::Infix)),
		(">".to_string(), (ifunction_binary_integers!(gt, bool) as IFunction, Position::Infix)),
		("<=".to_string(), (ifunction_binary_integers!(lteq, bool) as IFunction, Position::Infix)),
		(">=".to_string(), (ifunction_binary_integers!(gteq, bool) as IFunction, Position::Infix)),
		("<f".to_string(), (ifunction_binary_floats!(lt_float, bool) as IFunction, Position::Infix)),
		(">f".to_string(), (ifunction_binary_floats!(gt_float, bool) as IFunction, Position::Infix)),
		("<=f".to_string(), (ifunction_binary_floats!(lteq_float, bool) as IFunction, Position::Infix)),
		(">=f".to_string(), (ifunction_binary_floats!(gteq_float, bool) as IFunction, Position::Infix)),
		("++".to_string(), (concat as IFunction, Position::Infix)),
		("replace".to_string(), (replace as IFunction, Position::Classic)),
		("blen".to_string(), (blen as IFunction, Position::Classic)),
		("base_to_string".to_string(), (base_to_string as IFunction, Position::Classic)),
		("remove_fact".to_string(), (remove_fact as IFunction, Position::Classic)),
		("answer_subquestion".to_string(), (answer_subquestion as IFunction, Position::Classic)),
		("answer_once".to_string(), (answer_once as IFunction, Position::Classic)),
		("read_file_to_string".to_string(), (read_file_to_string as IFunction, Position::Classic)),
		("solve".to_string(), (solve as IFunction, Position::Classic)),
		("string".to_string(), (string as IFunction, Position::Classic)),
		("push".to_string(), (push1 as IFunction, Position::Classic)),
		("last".to_string(), (last1 as IFunction, Position::Classic)),
		("first".to_string(), (first1 as IFunction, Position::Classic)),
		("notempty".to_string(), (notempty as IFunction, Position::Classic)),
		("in".to_string(), (inlist as IFunction, Position::Infix)),
		("notin".to_string(), (notinlist as IFunction, Position::Infix)),
		("subseteq".to_string(), (subseteq as IFunction, Position::Infix)),
		("sort".to_string(), (sortlist as IFunction, Position::Classic)),
		("dedup".to_string(), (dedup as IFunction, Position::Classic)),
		("get".to_string(), (get_at as IFunction, Position::Classic)),
		("set".to_string(), (set_at as IFunction, Position::Classic)),
		("dist".to_string(), (dist as IFunction, Position::Classic)),
		("rhai_call".to_string(), (rhai_call as IFunction, Position::Classic)),
		// ("&".to_string(), (notequal as IFunction, Position::Infix)),
	]);


	for f in fs{
		let sid = psterms.add_ifunction(f.0.to_string(), Some((f.1).0), (f.1).1);
		fmap.insert(f.0, sid);
	}

	(psterms, fmap)
}
