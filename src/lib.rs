//! Math quiz library
//! 
//! Generates weighted-probability of pre-defined math problems with increased probability of selection based on frequency of incorrect answers and amount of times previously presented as well as the time required to correctly answer the most recent time

use std::fmt;
use rand::Rng;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use std::ops::{Add, Sub, Mul, Div};
use thiserror::Error;

/// The data type used for operands and results
type NumberType = u8;
/// The data type for problem selection using exponential moving average of time required to answer correctly and number of times answered incorrectly - must be floating point
type WeightType = f32;
/// The data type for the number of times answered incorrectly (limits maximum number of attempts)
type CountType = u32;

/// Define the maximum amount of time required to correctly answer (prevents single wrong answers from dominating the selection probability)
pub const MAX_ATTEMPT_TIME: Duration = Duration::from_secs(30);

/// Define the minimum weight for a problem (prevents problems that have been answered correctly many times from being ignored)
pub const BASELINE_WEIGHT: WeightType = 1.0;

/// Define the number of previous attempts to use for the exponential moving average of the time required to answer correctly (compute as factor k)
/// Note that since we multiply with duration we must specify the type here (f32) to avoid type mismatch errors. Alternative could be f64
pub const RECENT_TRIALS_WINDOW: f32 = 3.0;
pub const EMA_FACTOR: f32 = 2.0/(RECENT_TRIALS_WINDOW+1.0);

/// Weighting for incorrect answers
pub const ERROR_WEIGHT: WeightType = 10.0;

/// Weighting for time required to answer correctly
pub const TIME_WEIGHT: WeightType = 1.0;

/// String used to display addition
pub const ADD_SYMBOL: &str = "+";
pub const SUB_SYMBOL: &str = "-";
pub const MUL_SYMBOL: &str = "x";
pub const DIV_SYMBOL: &str = "\u{00f7}";

/// Compile time checks
const _: () = {
    assert!(EMA_FACTOR > 0.0 && EMA_FACTOR < 1.0, "EMA_FACTOR must be between 0 and 1");
};

/// Define the operators to be tested
/// 
/// Defines the mathematical operation to be performed on the operands. These are always binary operators
/// 
/// # Example
/// ```
/// # use math_quiz::{Operator, ADD_SYMBOL, SUB_SYMBOL, MUL_SYMBOL, DIV_SYMBOL};
/// assert_eq!(Operator::Add.apply(2, 3), 5); // 2+3 = 5
/// assert_eq!(Operator::Sub.apply(5, 3), 2); // 5-3 = 2
/// assert_eq!(Operator::Mul.apply(2, 3), 6); // 2*3 = 6
/// assert_eq!(Operator::Div.apply(6, 3), 2); // 6/3 = 2
/// assert_eq!(Operator::Add.symbol(), ADD_SYMBOL); // Returns the symbol for rendering the question
/// assert_eq!(Operator::Sub.symbol(), SUB_SYMBOL); // Returns the symbol for rendering the question
/// assert!(Operator::Add.is_valid(2, 3)); // Returns true for valid operands
/// assert!(Operator::Sub.is_valid(5, 3)); // Returns true for valid operands
/// assert!(!Operator::Sub.is_valid(2, 3)); // Returns false for invalid operands (negative result)
/// assert!(Operator::Div.is_valid(6, 3)); // Returns true for valid operands
/// assert!(!Operator::Div.is_valid(5, 3)); // Returns false for invalid operands (fractional result)
/// ```
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
}

impl Operator {
    // Returns the symbol for rendering the question
    pub fn symbol(&self) -> &'static str {
        match self {
            Operator::Add => ADD_SYMBOL,
            Operator::Sub => SUB_SYMBOL,
            Operator::Mul => MUL_SYMBOL,
            Operator::Div => DIV_SYMBOL,
        }
    }

    // Executes the operator as a binary function
    pub fn apply(&self, a: NumberType, b: NumberType) -> NumberType {
        match self {
            Operator::Add => NumberType::add(a, b),
            Operator::Sub => NumberType::sub(a, b), // Safe assuming valid() was checked
            Operator::Mul => NumberType::mul(a, b),
            Operator::Div => NumberType::div(a, b), // Safe assuming b != 0 and perfect division
        }
    }

    // Ensures the inputs will output a whole, non-negative integer
    pub fn is_valid(&self, a: NumberType, b: NumberType) -> bool {
        match self {
            Operator::Add => true,
            // Prevents negative results
            Operator::Sub => a >= b, 
            Operator::Mul => true,
            // Prevents division by zero and fractional/decimal results
            Operator::Div => b != 0 && a.is_multiple_of(b),
        }
    }    
}

/// Problem
/// 
/// Describes a simple mathematical problem composed of two operands and an operator
/// It also stores information about how often the problem has been presented and how often it has been incorrectly answered
/// In addition, it stores the total time in seconds used to answer the problem
/// 
/// # Example
/// ```
/// # use math_quiz::{Problem, Operator, BASELINE_WEIGHT, ERROR_WEIGHT, TIME_WEIGHT};
/// # use std::time::Duration;
/// // Start with simple 6 multiplied by 3 problem with 0 wrong attempts and 10 seconds to answer correctly
/// let problem = Problem::new([6, 3], Operator::Mul, 0, Duration::from_secs(10)).unwrap();
/// assert_eq!(problem.get_op(), Operator::Mul);
/// assert_eq!(problem.get_num_wrong(), 0);
/// assert_eq!(problem.get_time(), Duration::from_secs(10));
/// assert_eq!(problem.get_weight(), BASELINE_WEIGHT + 0.0 * ERROR_WEIGHT + 10.0 * TIME_WEIGHT);
/// assert_eq!(problem.to_string(), concat!("6 x 3 = "));
/// 
/// ```
#[derive(Serialize, Deserialize, Debug)]
pub struct Problem {
    /// Exactly two operands for each operation
    operands: [NumberType;2],
    /// The mathematical operation
    operator: Operator,    
    /// Number of times answered incorrectly before finally entering correct answer on most recent attempt
    num_wrong: CountType,
    /// The expoential moving average of the time required to answer the problem correctly
    ema_time: Duration,
}

/// Error type for problems
#[derive(Debug, Error)]
pub enum ProblemError {
    /// Invalid operands for the given operator
    #[error("Invalid operands for the given operator: {operand1:?}, {operand2:?} with operator {operator:?}", operand1 = operands[0], operand2 = operands[1])]
    InvalidOperands {
        operands: [NumberType; 2],        
        operator: Operator,
    },
    /// This is a catch-all for any other errors that may occur
    #[error("Undefined error")]
    UndefinedError,
}

/// Display the problem in human-readable text
impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{lhs} {op} {rhs} = ", lhs = self.operands[0], op = self.operator.symbol(), rhs = self.operands[1])
    }
}

impl Problem {
    /// Create a new problem with two specified operands, an operator, initial starting wrong attempts, and ema_time
    /// Returns an error if the operands are invalid for the given operator, e.g. results in negative or fractional answers
    pub fn new(operands: [NumberType;2], operator: Operator, num_wrong: CountType, ema_time: Duration) -> Result<Self, ProblemError> {
        // Validate the operands for the given operator
        if !operator.is_valid(operands[0], operands[1]) {
            return Err(ProblemError::InvalidOperands {
                operands,
                operator,
            });
        }
        Ok(Self {  
            operands,                         
            operator,
            num_wrong,             
            ema_time,
         })
    } 

    /// Return a weight for the problem based on the number of times answered incorrectly and the amount of time required to answer correctly    
    pub fn get_weight(&self) -> WeightType {        
        let weight = BASELINE_WEIGHT + self.num_wrong as WeightType * ERROR_WEIGHT + self.ema_time.as_secs() as WeightType * TIME_WEIGHT;
        assert!(weight>0.0, "Weight must be positive");
        weight
    }
    
    /// Return the operator used for this problem
    /// This is used if condition is required for all operators to be presented at least once
    pub fn get_op(&self) -> Operator {
        self.operator
    }

    /// Check an attempt. If incorrect, increase number of wrong answers. If correct, update the EMA of time to get to the correct answer. If correct, return true, otherwise return false
    pub fn check_guess(&mut self, guess: NumberType, elapsed_time: Duration) -> bool {
        // Compute answer
        let answer = self.operator.apply(self.operands[0], self.operands[1]);

        // Check answer
        if answer == guess {
            // Update new EMA of time to answer correctly
            self.ema_time = elapsed_time.mul_f32(EMA_FACTOR).add(self.ema_time.mul_f32(1.0 - EMA_FACTOR));            
            // Returns true for correct answer
            true
        } else {            
            // Increase number incorrect by 1
            self.num_wrong+=1;
            //return false for incorrect answer
            false
        }
    }

    /// Return the amount of time required to answer the question correctly the last time
    /// TODO: Find where it is used
    pub fn get_time(&self) -> Duration {
        self.ema_time
    }

    /// Return the number of times answered incorrectly
    pub fn get_num_wrong(&self) -> CountType {
        self.num_wrong
    }
}

/// Initialize a problem set
/// Start with addition from 0..12
/// Then add subtraction from 0..12 with only non-negative results
/// Then add multiplication from 1..12
/// Finally add division from 1..12 with only whole number results
pub fn init_problems(problems: &mut Vec<Problem>) {
    add_addition(problems);
    add_subtraction(problems);
    add_mult(problems);
    add_division(problems);
}

/// Add basic addition problems using default always correct with maximum time
/// 
/// # Example
/// ```
/// # use math_quiz::{Problem, add_addition};
/// let mut problems: Vec<Problem> = Vec::new();
/// add_addition(&mut problems);
/// assert_eq!(problems.len(), 12*13);
/// ```
pub fn add_addition(problems: &mut Vec<Problem>) {
    // Start with addition problems of 0..12
    for x in 1..=12 {
        for y in 0..=12 {
            // Since we are constructing this, we can assume that the operands are valid for addition, so we can use unwrap() to avoid handling the error
            problems.push(Problem::new([x, y], Operator::Add, 0, MAX_ATTEMPT_TIME).unwrap());            
        }
    }   
}

/// Add basic subtraction problems using default always correct with maximum time
/// 
/// # Example
/// ```
/// # use math_quiz::{Problem, add_subtraction};
/// let mut problems: Vec<Problem> = Vec::new();
/// add_subtraction(&mut problems);
/// assert_eq!(problems.len(), (13-1)/2*(13+2));
/// ```
pub fn add_subtraction(problems: &mut Vec<Problem>) {
    // Start with basic subtractions problems of 0..12, non-negative
    for x in 1..=12 {
        for y in 0..=x {
            // Since we are constructing this, we can assume that the operands are valid for subtraction, so we can use unwrap() to avoid handling the error
            problems.push(Problem::new([x,y], Operator::Sub, 0, MAX_ATTEMPT_TIME).unwrap());
        }
    }    
}

/// Add basic multiplication problems
/// 
/// # Example
/// ```
/// # use math_quiz::{Problem, add_mult};
/// let mut problems: Vec<Problem> = Vec::new();
/// add_mult(&mut problems);
/// assert_eq!(problems.len(), 12*13);
/// ```
pub fn add_mult(problems: &mut Vec<Problem>) {
    // Make multiplication problems from 1 to 12 times 1 to 12
    for x in 1..=12 {
        for y in 0..=12 {
            // Since we are constructing this, we can assume that the operands are valid for multiplication, so we can use unwrap() to avoid handling the error
            problems.push(Problem::new([x,y], Operator::Mul, 0, MAX_ATTEMPT_TIME).unwrap());           
        }
    }    
}

/// Add basic division problems
/// 
/// # Example
/// ```
/// # use math_quiz::{Problem, add_division};
/// let mut problems: Vec<Problem> = Vec::new();
/// add_division(&mut problems);
/// assert_eq!(problems.len(), 12*12); // 12*12 = 144
/// ```
pub fn add_division(problems: &mut Vec<Problem>) {
    // Make division problems from 1 to 12 divided by 1 to 12, only whole numbers
    for x in 1..=12 {
        for y in 1..=12 {            
            // Since we are constructing this, we can assume that the operands are valid for division, so we can use unwrap() to avoid handling the error
            problems.push(Problem::new([x*y, x], Operator::Div, 0, MAX_ATTEMPT_TIME).unwrap());            
        }
    }    
}

/// Randomly select a problem from the problem set based on weight. Returns the index into the vector rather than the problem itself to avoid cloning the problem.
/// 
/// # Example
/// ```
/// # use math_quiz::{Problem, add_addition, select_problem};
/// let mut problems: Vec<Problem> = Vec::new();
/// add_addition(&mut problems);
/// let index = select_problem(&problems);
/// assert!(index < problems.len());
/// ```
pub fn select_problem(problems: &[Problem]) -> usize {
    // Compute maximum weight as sum of all weights from each problem
    let max_weight: WeightType = problems.iter().map(|p| p.get_weight()).sum();
    // Get random number from 0 to max_weight, inclusive
    let pick = rand::thread_rng().gen_range(0.0..=max_weight);
    // Now pick the problem by finding the first problem after the cumulative weight of all prior problems exceeds the random number picked
    let mut score: WeightType = 0.0;
    for (p, problem) in problems.iter().enumerate() {
        score+=problem.get_weight();
        if score>=pick {
            return p;
        }
    }
    // Otherwise return final problem
    problems.len()-1
}

#[cfg(test)]
mod tests {
    // Pull all references above into scope
    use super::*; 

    /// Setup 3 questions with known weights and test for expected distribution
    #[test]
    fn test_select() {
        let mut num_selected: [i32;3] = [0, 0, 0];
        let problems = vec![Problem::new([7,6],Operator::Add,30,Duration::from_secs(30)).unwrap(),
        Problem::new([1,1],Operator::Add,10,Duration::from_secs(20)).unwrap(),
        Problem::new([7,6],Operator::Add,5,Duration::from_secs(5)).unwrap(),
        ];
        
        for _rep in 0..10000000 {
            num_selected[select_problem(&problems)]+=1;
        }
        eprintln!("{:?}", num_selected);
        assert!(i32::abs(num_selected[0]/100000 - 66)<=1, "We expected 66% for first problem");
        assert!(i32::abs(num_selected[1]/100000 - 23)<=1, "Expected 30% for second problem");
        assert!(i32::abs(num_selected[2]/100000 - 11)<=1, "Expected 10% for third problem");
    }
}