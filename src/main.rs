use std::mem;
use clap::Parser;

use bootfrost::solver::*;
use bootfrost::strategies::strategies::Strategy;
use bootfrost::strategies::rhai_runtime::RhaiRuntime;

use std::fs::File;
use std::io::Write;
use std::path::Path;

//use bootfrost::solverlog::*;

#[derive(Parser,Default,Debug)]
#[clap(author="Aleksandr Larionov", version, about="Bootfrost Solver")]
struct Arguments{
	#[clap(short, long)]
	/// Path to the file containing the formula
	formula: String,

	#[clap(short, long)]
	/// Strategy: "plain", "general", "manualfirst", "manualbest" or path to the file containing the user strategy
	strategy: String,

	#[clap(short, long)]
	/// Maximum number of steps
	limit: usize,

	#[clap(short, long)]
	/// JSON logging
	json: bool,

	#[clap(short = 'u', long = "user-ifuncs")]
	/// Path to a Rhai script that defines user functions (e.g. `a_star_step`)
	/// accessible from the formula via the corresponding ifunction names.
	user_ifuncs: Option<String>,
}


fn main() {

	let args = Arguments::parse();
	println!("{:?}", args);

	// Install the user-supplied Rhai script (if any) into the global slot
	// before constructing the solver, so the ifunction wrappers can reach
	// it from inside the inference loop.
	if let Some(path) = &args.user_ifuncs {
		let rt = RhaiRuntime::from_file(path).unwrap_or_else(|e| {
			panic!("failed to load Rhai script '{}': {}", path, e);
		});
		rt.install_global().unwrap_or_else(|e| {
			panic!("failed to install Rhai runtime: {}", e);
		});
	}

	let s = match args.strategy.as_str(){
		"plain" => Strategy::PlainShift,
		"general" => Strategy::General,
		"manualfirst" => Strategy::ManualFirst,
		"manualbest" => Strategy::ManualBest,
		_ => {
			panic!("Invalid strategy name. Type plain, general, manualfirst or manualbest.");
		},
	};

	let mut solver = Solver::parse_file(&args.formula, s);
	solver.print();
	let r = solver.solver_loop(args.limit);
	solver.slog.set_result(format!("{:?}",r));
	if args.json{
		let j = serde_json::to_string_pretty(&solver.slog).unwrap();
		//println!("\n\n---- JSON LOG ----\n {}", j);

		let json_path = Path::new(&args.formula).with_extension("json");
	    let mut data_file = File::create(&json_path).expect("creation failed");
	    data_file.write(j.as_bytes()).expect("write failed");
	    println!("---- JSON has been saved ----\n");

	}

}
