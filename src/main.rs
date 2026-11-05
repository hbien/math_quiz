use std::io::{self};
use std::io::Write;
use std::fs;
use std::time::Instant;
use math_quiz::Problem;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(name = "Math Quiz")]
#[command(version, about, long_about=None)]
struct Args {
    /// Specify configuration filename
    #[arg(short, long, default_value = "math_quiz.ini")]
    config: PathBuf,
    /// Reset progress
    #[arg(short, long, default_value_t=false)]
    reset: bool,
    /// Specify minimum number of questions per session
    #[arg(short='q', long, default_value_t=10)]    
    min_questions: usize,
    /// Specify the maximum number of questions per session
    #[arg(short='Q', long, default_value_t=30)]
    max_questions: usize,
    /// Specify maximum time (in seconds) to answer a question in seconds to be counted as correct
    #[arg(short='t', long, default_value_t=2)]
    max_time_secs: u64,
    /// Specify number of consecutive correct answers to finish session
    #[arg(short='n', long, default_value_t=5)]
    consecutive_correct: usize,
    /// Require all operators to be tested at least once per session before early termination
    #[arg(short='a', long, default_value_t=false)]
    require_all_ops: bool,
    /// Enable verbose output for debugging
    #[arg(short='v', long, default_value_t=false)]
    verbose: bool,
    /// Add questions to question bank
    #[command(subcommand)]
    cmd: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Add new questions
    Add {
        /// Question type
        #[arg(short, long)]
        question_type: String,
    },
}

fn main() -> std::io::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Print out the arguments for debugging
    if args.verbose {
        println!("Arguments: {:?}", args);
    }

    // List of problems
    let mut problems: Vec<Problem> = Vec::new();

    // Attempt to load question bank from config file if specified
    load_progress(&mut problems, &args.config);

    // If requested, reset progress regardless whether successfully loaded or not
    if args.reset {
        println!("Resetting progress");
        math_quiz::init_problems(&mut problems);
        // Save new reset, ignore any problems
        let _ = save_progress(&problems, &args.config);
    }
    
    // Add more questions if requested
    match &args.cmd {
        Some(Commands::Add{question_type}) => {
            match question_type.as_str() {
                "+" | "plus" => math_quiz::add_addition(&mut problems),
                "-" | "minus" => math_quiz::add_subtraction(&mut problems),
                "x" | "*" | "multiplication" => math_quiz::add_mult(&mut problems),
                "/" | "division" => math_quiz::add_division(&mut problems),
                _ => eprintln!("Unknown question type to add: {}", question_type),         
            }
            // Save new questions and then quit
            return save_progress(&problems, &args.config);        
        },
        None => {}        
    }

    // Ensure each math operation in question set is seen at least once
    let mut op_seen: HashMap<math_quiz::Operator, bool> = problems.iter().map(|p| (p.get_op(), false)).collect();
    
    for p in &problems {
        op_seen.insert(p.get_op(), false);
    }    

    // Counter for number of consecutive correct answers
    let mut num_correct =0;

    // Count total number of questions
    let mut num_questions = 0;

    // These should be configurable
    let max_time = 2;    

    // Loop no more than max_questions
    while num_questions <= args.max_questions && (
                                                    // Check for early termination only if minimum number of questions have been reached
                                                    num_questions < args.min_questions || 
                                                    // Early termination (evaluates to true) if consecutive correct answers reached and all operators seen at least once
                                                    num_correct<args.consecutive_correct || 
                                                    ( // Short circuit if require_all_ops is false, then don't check for all operators seen
                                                        args.require_all_ops && op_seen.iter().any(|(_op, seen)| !seen)
                                                    )
                                                ) {
        // Select problem based on number of times presented, number incorrect, and time to answer correctly
        let prob = math_quiz::select_problem(&problems);

        // Set flag for operation seen
        op_seen.insert(problems[prob].get_op(), true);

        // Tally new question
        num_questions+=1;

        // Loop until correct answer entered

        // Start the timer
        let timer = Instant::now();

        loop {            
            // Print problem        
            print!("#{}: {}", num_questions, problems[prob]);
            // Flush since no endline
            io::stdout().flush().unwrap();
        
            // Get answer            
            let mut guess = String::new();
            io::stdin()
            .read_line(&mut guess)
            .expect("Unable to read from stdin");

            // Try to convert answer to numeric value
            let guess: u8 = match guess
            .trim()
            .parse() {
                Ok(num) => num,
                Err(_) => {
                    println!("{guess} is not a valid number!");
                    // Try again - does not count as incorrect answer
                    continue;
                }
            };

            // Stop the clock
            let elapsed_time = timer.elapsed();

            // Check and see if correct answer
            if problems[prob].check_guess(guess, elapsed_time) {            
                println!("Correct! It took you {} seconds to solve.", elapsed_time.as_secs());
                if elapsed_time.as_secs()<=max_time {
                    num_correct+=1;
                }
                break;          
            } else {
                println!("Sorry, that is not correct.");
                // Reset culmulative counter
                num_correct=0;
            }            
        }        
    }
   
    println!("Congratulations! You have finished for today.");
    // return
    save_progress(&problems, &args.config)
}

/// Load question bank from file, and reset if any errors encountered
fn load_progress(problems: &mut Vec<Problem>, path: &PathBuf) {    
    // Read entire file into string
    let saved_progress = fs::read_to_string(path);
    // Replace problems with config file; initialize if any errors encountered
    if let Ok(saved_progress) = saved_progress {
        println!("Reading progress from {}", path.display());
        let new_problems: Vec<Problem> = serde_json::from_str(&saved_progress).expect("Error deserializing problems");
        *problems=new_problems;
    } else {        
        println!("Unable to open progress file - resetting.");
        // Start with list of problems        
        math_quiz::init_problems(problems);            
    }
}

/// Save progress to specified file or math_quiz.ini if not specified
fn save_progress(problems: &Vec<Problem>, path: &PathBuf) -> io::Result<()> {    
    println!("Saving progress to {}", path.display());
    // Save progress
    fs::write(path, serde_json::to_string(&problems).expect("Error serializing problems")).expect("Error saving progress file");
    //return
    Ok(())
}