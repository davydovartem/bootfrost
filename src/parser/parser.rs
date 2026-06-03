
use crate::plain::*;
use lalrpop_util::ParseError;



use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use std::fmt;


//https://docs.python.org/3/reference/lexical_analysis.html#indentation

pub fn align_modulo(n:i64, m: i64) -> i64{
    n + (m - (n % m))
}


pub struct PfLine{
	pub line: String,
	pub indent: i64,
}

impl PfLine{
	pub fn new(line: String) -> PfLine{
		let s = line.clone();
		let mut indent = 0;
        
        for c in line.chars(){
            if c == ' '{
                indent = indent + 1;
            }
            if c == '\t'{
                indent = align_modulo(indent, 8);
            }
            if c != ' ' && c != '\t' {
                break;
            }
        }
		PfLine{line:s, indent:indent}
	}
}

impl fmt::Display for PfLine{
    fn fmt (&self, fmt: &mut fmt::Formatter) -> fmt::Result{
		write!(fmt,"{}:{}",self.indent, self.line)
    }
}


fn proc(lines: &Vec<PfLine>, k: &mut usize, stack_indents: &mut Vec<i64>) -> Vec<PlainFormula>{
    let mut res = vec![];
    if *k >= lines.len(){
        return res;
    }

    while lines[*k].indent > *stack_indents.last().unwrap(){
        stack_indents.push(lines[*k].indent);
        println!("{}",&lines[*k].line);
        let mut pf = match crate::parser::tqfline::TqfLineParser::new().parse(&lines[*k].line){
            Ok(pf) => pf,
            Err(e) => {
                eprintln!("\n=========================================================");
                eprintln!("PARSER ERROR in formula line #{}", *k);
                eprintln!("=========================================================");
                eprintln!("Text of line:\n  {}", lines[*k].line);
                eprintln!("\n{}", format_parse_error(&lines[*k].line, e));
                panic!("Parse error in formula line #{}", *k);
            }
        };
        *k = *k + 1;
        if *k >= lines.len(){
            res.push(pf);
            return res;
        }
        pf.next = proc(lines, k, stack_indents);

        res.push(pf);
        if *k >= lines.len(){
            return res;
        }
    }
    //dbg!(&stack_indents);
    if !stack_indents.contains(&lines[*k].indent){
        panic!("Indentation error");
    }

    stack_indents.pop();
    return res;
}

/// Convert byte position to line-column pair in a formula line.
/// Always (1, byte+1), because the parser applies to one line after concatenation.
fn byte_to_linecol(s: &str, byte: usize) -> (usize, usize){
    // If location exceeds EOF, clamp to the end of the line.
    let clamped = byte.min(s.len());
    (1, clamped + 1)
}

/// Draws a highlighted fragment of the string with the error position.
fn highlight_location(s: &str, byte: usize) -> String{
    let clamped = byte.min(s.len());
    let col = clamped + 1; // 1-based for display

    // Show window ~60 characters around the error position
    let window: usize = 60;
    let start = clamped.saturating_sub(window);
    let end = (clamped + window).min(s.len());
    let slice = &s[start..end];

    // Print the fragment and the arrow; if the window is truncated on the left/right, add "..."
    let prefix = if start > 0 { "..." } else { "" };
    let suffix = if end < s.len() { "..." } else { "" };
    let arrow_col = (clamped - start) + prefix.len() + 1;

    let mut underline = String::with_capacity(prefix.len() + slice.len() + 4);
    underline.push_str(&" ".repeat(prefix.len()));
    for (i, c) in slice.char_indices(){
        if i + start == clamped{
            // Mark the character itself with an arrow
            let width = c.len_utf8();
            underline.push_str(&"^".repeat(width.max(1)));
        }else{
            // Spaces of corresponding width (for multi-byte characters, multiple spaces)
            let width = c.len_utf8();
            underline.push_str(&" ".repeat(width));
        }
    }
    underline.push_str(suffix);

    format!("  {}{}{}\n  {}\n  (position: column {})", prefix, slice, suffix, underline, col)
}

/// Formats ParseError from LALRPOP into a human-readable message.
/// Token type is parametrized — we only need its string representation.
pub fn format_parse_error<T: ToString>(line: &str, e: ParseError<usize, T, &str>) -> String{
    let expected_str = |expected: &Vec<String>| -> String {
        if expected.is_empty(){
            "(nothing expected)".to_string()
        }else{
            expected.iter()
                .map(|s| format!("'{}'", s))
                .collect::<Vec<_>>()
                .join(", ")
        }
    };

    match e{
        ParseError::InvalidToken{ location } => {
            let (l, c) = byte_to_linecol(line, location);
            format!("Unexpected token on line {}, column {} (byte #{}).\n{}\nExpected: {}",
                l, c, location, highlight_location(line, location), expected_str(&Vec::new()))
        },
        ParseError::UnrecognizedEOF{ location, expected } => {
            let (l, c) = byte_to_linecol(line, location);
            format!("Unexpected end of line on line {}, column {} (byte #{}).\n{}\nExpected one of: {}",
                l, c, location, highlight_location(line, location), expected_str(&expected))
        },
        ParseError::UnrecognizedToken{ token: (start, tok, _end), expected } => {
            let (l, c) = byte_to_linecol(line, start);
            format!("Unrecognized token '{}' on line {}, column {} (byte #{}).\n{}\nExpected one of: {}",
                tok.to_string(), l, c, start, highlight_location(line, start), expected_str(&expected))
        },
        ParseError::ExtraToken{ token: (start, tok, _end) } => {
            let (l, c) = byte_to_linecol(line, start);
            format!("Extra token '{}' on line {}, column {} (byte #{}).\n{}",
                tok.to_string(), l, c, start, highlight_location(line, start))
        },
        ParseError::User{ error } => {
            format!("Parser error: {}", error)
        },
    }
}



fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

pub fn file_to_pflines(path: &str) -> Vec<PfLine>{
    let mut true_lines: Vec<PfLine> = vec![];
    let mut buff = String::new();
    let mut flag = false;

    if let Ok(lines) = read_lines(path) {
        for line in lines {
            if let Ok(origin_line) = line {
                prepare_lines_string(&origin_line, &mut true_lines, &mut buff, &mut flag);
            }
        }
        return true_lines;
    }else{
        panic!("");
    }
}

pub fn string_to_pflines(s: &str) -> Vec<PfLine>{
    let mut true_lines: Vec<PfLine> = vec![];
    let mut buff = String::new();
    let mut flag = false;

    for origin_line in s.lines(){
        prepare_lines_string(&origin_line, &mut true_lines, &mut buff, &mut flag);
    }

    return true_lines;
}

pub fn prepare_lines_string(
        origin_line: &str,
        true_lines: &mut Vec<PfLine>,
        buff: &mut String,
        flag: &mut bool){

    let line0 = if let Some((s,_)) = origin_line.split_once("#"){
        s
    }else{
        &origin_line
    };

    let trimmed = line0.trim_end();

    if trimmed.is_empty(){
        return;
    }

    let (payload, line_continues) = if trimmed.ends_with('~'){
        (&trimmed[..trimmed.len() - '~'.len_utf8()], true)
    }else{
        (trimmed, false)
    };

    if payload.trim().is_empty(){
        return;
    }

    buff.push_str(payload);
    if !line_continues{
        let pfline = PfLine::new(buff.clone());
        true_lines.push(pfline);
        *buff = String::new();
        *flag = false;
    }else{
        *flag = false;
    }

}


fn pflines_to_plainformula(pflines: Vec<PfLine>) -> PlainFormula{
    let mut k = 0;
    let mut stack_indents = vec![-1];
    let mut res = PlainFormula{quantifier:"!".to_string(), vars: vec![], conjunct: vec![], commands:vec![], next: vec![]};
    res.next = proc(&pflines, &mut k, &mut stack_indents);
    res
}


pub fn parse_string(s: &str) -> PlainFormula{
    let pflines = string_to_pflines(s);
    pflines_to_plainformula(pflines)
}


pub fn parse_file(path: &str) -> PlainFormula{
    let pflines = file_to_pflines(path);

    // for x in &pflines{
    //     println!("line: {}",&x.line);
    // }

    pflines_to_plainformula(pflines) 
}




//
