//! Math quiz library
//! 
//! Generates weighted-probability of pre-defined math problems with increased probability of selection based on frequency of incorrect answers and amount of times previously presented as well as the time required to correctly answer the most recent time

use std::fmt;
use rand::Rng;
use rand::distributions::{Distribution, Uniform};
use std::time::Duration;
use serde::{Serialize, Deserialize};

/// MathOp
/// 
/// Mathematical operators
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum MathOp {
    Plus,
    Minus,
    Multiply,
    //Divide
}

impl std::fmt::Display for MathOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return write!(f, "{}", match self {
            MathOp::Plus => "+",
            MathOp::Minus => "-",
            MathOp::Multiply => "x",
            //MathOp::Divide => "\u{00f7}"            
        });
    }
}

/// Problem
/// 
/// Describes a simple mathematical problem composed of two operands and an operator
/// It also stores information about how often the problem has been presented and how often it has been correctly answered
/// In addition, it stores the time in seconds required to answer the problem
#[derive(Serialize, Deserialize, Debug)]
pub struct Problem {
    operands: [u8;2],
    operator: MathOp,
    answer: u8,
    num_wrong: u16,    
    latest_time: Duration
}

/*
impl Rng for Problem {

} */

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return write!(f, "{} {} {} = ", self.operands[0], self.operator, self.operands[1]);
    }
}

impl Problem {
    pub fn new(operands: [u8;2], operator: MathOp, num_wrong: u16, latest_time: Duration) -> Self {        
        Self {  
            operands,             
            answer: match operator {
                    MathOp::Plus => operands[0]+operands[1],
                    MathOp::Minus => operands[0]-operands[1],
                    MathOp::Multiply => operands[0]*operands[1] 
                },
            operator,
            num_wrong,             
            latest_time }
    } 

    fn get_score(&self) -> f32 {
        return self.num_wrong as f32 * 30.0 + self.latest_time.as_secs() as f32;
    }

    pub fn get_op(&self) -> MathOp {
        return self.operator;
    }

    pub fn check_guess(&mut self, guess: u16, elapsed_time: Duration) -> bool {
        if self.answer as u16==guess {
            // Store time required to answer correctly            
            self.latest_time=elapsed_time;
            if self.num_wrong>1 {self.num_wrong-=1;}
            return true;
        } else {            
            self.num_wrong+=1;
            return false;
        }
    }

    pub fn get_time(&self) -> Duration {
        return self.latest_time;
    }
}

/// Initialize a problem set
/// Start with addition from 0..10
/// Then add subtraction from 0..10 with only non-negative results
pub fn init_problems(problems: &mut Vec<Problem>) {
    add_addition(problems);
    add_subtraction(problems);
    return add_mult(problems);
}

/// Add basic addition problems
pub fn add_addition(problems: &mut Vec<Problem>) {
    // Start with addition problems of 0..10
    for x in 1..=15 {
        for y in 0..=13 {
            problems.push(Problem::new([x, y], MathOp::Plus, 0, Duration::from_secs(5)));
            if x!=y {
                problems.push(Problem::new([y, x], MathOp::Plus, 0, Duration::from_secs(5)));
            }
        }
    }   
}

/// Add basic subtraction problems
pub fn add_subtraction(problems: &mut Vec<Problem>) {
    // Start with basic subtractions problems of 0..10
    for x in 0..=15 {
        for y in 1..x {
            problems.push(Problem::new([x,y], MathOp::Minus, 0, Duration::from_secs(10)));
        }
    }    
}

/// Add basic multiplication problems
pub fn add_mult(problems: &mut Vec<Problem>) {
    // Start with basic subtractions problems of 0..10
    for x in 1..=5 {
        for y in 1..=3 {
            problems.push(Problem::new([x,y], MathOp::Multiply, 0, Duration::from_secs(15)));
        }
    }    
}

pub fn select_problem(problems: &Vec<Problem>) -> usize {
    // Compute maximum score
    let max_score: f32 = problems.iter().map(|p| p.get_score()).sum();
    // Get random number from 0 to maximum_score, inclusive
    let pick = rand::thread_rng().gen_range(0.0..=max_score);
    // Now pick the problem
    let mut score: f32 = 0.0;
    for p in 0..problems.len() {
        score+=problems[p].get_score();
        if score>=pick {
            return p;
        }
    }
    // Otherwise return final problem
    return problems.len()-1;
}

/// Sort problems for presentation serially using random process which favors incorrectly answered questions as well as
/// quesions which took a long time to answer
pub fn sort_problems(problems: &mut Vec<Problem>) {
    let rng = Uniform::from(0..=10000);
    
}
#[cfg(test)]
mod tests {
    // Pull all references above into scope
    use super::*; 

    /// Setup 3 questions with known weights and test for expected distribution
    #[test]
    fn test_select() {
        let mut num_selected: [i32;3] = [0, 0, 0];
        let mut problems: Vec<Problem> = Vec::new();
        // Add three simple problems
        problems.push(Problem::new([7,6],MathOp::Plus,30,Duration::from_secs(30)));
        problems.push(Problem::new([1,1],MathOp::Plus,10,Duration::from_secs(20)));
        problems.push(Problem::new([7,6],MathOp::Plus,5,Duration::from_secs(5)));
        for _rep in 0..10000000 {
            num_selected[select_problem(&problems)]+=1;
        }
        eprintln!("{:?}", num_selected);
        assert!(i32::abs(num_selected[0]/100000 - 66)<=1, "We expected 66% for first problem");
        assert!(i32::abs(num_selected[1]/100000 - 23)<=1, "Expected 30% for second problem");
        assert!(i32::abs(num_selected[2]/100000 - 11)<=1, "Expected 10% for third problem");
    }
}