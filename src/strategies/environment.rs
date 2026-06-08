use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::misc::*;
use crate::term::*;
use crate::base::*;
use crate::answer::*;
use crate::strategies::attributes::*;

pub type RhaiCallCache = HashMap<(TermId, TermId), TermId>;

pub struct PEnv<'a>{
	pub psterms: &'a mut PSTerms,
	pub base: &'a mut Base,
	pub answer: &'a Answer,
	pub attributes: &'a mut Attributes,
	pub bid: BlockId,
	pub answer_subquestions: Vec<usize>,
	pub answer_once: bool,
	pub rhai_call_cache: &'a mut RhaiCallCache,
}
