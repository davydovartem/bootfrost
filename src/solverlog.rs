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
	//pub completed: bool
}

#[derive(Serialize)]
pub struct SolverLog{
	pub formula: Option<JsonFormula>,
	pub log: Vec<StepItem>,
	pub result: String,
	//pub curr_step: usize
}

impl SolverLog{
	pub fn new() -> SolverLog{
		SolverLog{
			formula: None,
			log: vec![],
			result: "".to_string()
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

