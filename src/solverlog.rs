use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct JsonTerm{
	pub name: String,
	#[serde(skip_serializing_if = "Vec::is_empty", default)]
	pub args: Vec<JsonTerm>,
}

impl JsonTerm{
	pub fn leaf(name: String) -> JsonTerm{
		JsonTerm{name, args: vec![]}
	}

	pub fn node(name: String, args: Vec<JsonTerm>) -> JsonTerm{
		JsonTerm{name, args}
	}
}

#[derive(Clone, Serialize)]
pub struct JsonFormula{
	pub qtype: String,
	pub vars_list: Vec<JsonTerm>,
	pub atoms_list: Vec<JsonTerm>,
	pub children: Vec<JsonFormula>,
}

#[derive(Clone, Serialize)]
pub struct JsonBaseItem{
	pub atom: JsonTerm,
	pub deleted: bool,
}

#[derive(Serialize)]
pub struct StepItem{
	pub step: usize,
	pub question: usize,
	pub answer: String,
	pub atoms_added: Vec<JsonTerm>,
	pub atoms_used: Vec<JsonTerm>,
	pub base: Vec<JsonBaseItem>,
	pub current_base: usize,
	//pub completed: bool
}

#[derive(Serialize)]
pub struct RefutedBaseStats{
	pub base_number: usize,
	pub step_start: usize,
	pub step_end: usize,
	pub steps_to_refute: usize,
	pub time_ms: u128,
	pub base_len_start: usize,
	pub base_len_end: usize,
	pub questions_added: usize,
	pub atoms_added: usize,
	pub atoms_removed: usize,
}

#[derive(Serialize)]
pub struct SolverLog{
	pub formula: Option<JsonFormula>,
	pub log: Vec<StepItem>,
	pub result: String,
	pub refuted_bases: Vec<RefutedBaseStats>,
	//pub curr_step: usize
}

impl SolverLog{
	pub fn new() -> SolverLog{
		SolverLog{
			formula: None,
			log: vec![],
			result: "".to_string(),
			refuted_bases: vec![]
			//curr_step: 0
		}
	}

	pub fn is_empty(&self) -> bool{
		self.log.is_empty()
	}

	pub fn new_step(&mut self, n: usize){
		let x = StepItem{
			step:n,
			question:0,
			answer: "".to_string(),
			atoms_added: vec![],
			atoms_used: vec![],
			base: vec![],
			current_base: 0,
			//completed: false
		};
		self.log.push(x);
	}

	pub fn set_formula(&mut self, f: JsonFormula){
		self.formula = Some(f);
	}

	// set question and answer
	pub fn set_qa(&mut self, q: usize, a: String){
		let x = self.log.last_mut().unwrap();
		x.question = q;
		x.answer = a;
	}

	pub fn set_current_base(&mut self, bid: usize){
		if let Some(x) = self.log.last_mut(){
			x.current_base = bid;
		}
	}

	pub fn add_refuted_base(&mut self, s: RefutedBaseStats){
		self.refuted_bases.push(s);
	}

	pub fn set_atoms(&mut self, a_a: Vec<JsonTerm>, a_u: Vec<JsonTerm>){
		let x = self.log.last_mut().unwrap();
		x.atoms_added = a_a;
		x.atoms_used = a_u;
	}

	pub fn set_base(&mut self, b: Vec<JsonBaseItem>){
		let x = self.log.last_mut().unwrap();
		x.base = b;
	}

	pub fn set_result(&mut self, r: String){
		self.result = r;
	}
}

