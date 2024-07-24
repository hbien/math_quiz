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
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    /// Reset progress
    #[arg(short, long, default_value_t=false)]
    reset: bool,
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
                _ => eprintln!("Unknown question type to add: {}", question_type),         
            }
            // Save new questions and then quit
            return save_progress(&problems, &args.config);        
        },
        None => {}        
    }

    // Ensure each math operation in question set is seen at least once
    let mut op_seen = HashMap::new();

    for p in &problems {
        op_seen.insert(p.get_op(), false);
    }    

    // Counter for number of consecutive correct answers
    let mut num_correct =0;

    // Count total number of questions
    let mut num_questions = 0;

    // Loop until 5 consecutive correct answers in less than 2 seconds
    while num_questions<30 && (num_correct<5 || op_seen.iter().any(|x| *x.1==false)) {       
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
            print!("#{}: {}", &num_questions, problems[prob]);
            // Flush since no endline
            io::stdout().flush().unwrap();
        
            // Get answer            
            let mut guess = String::new();
            io::stdin()
            .read_line(&mut guess)
            .expect("Unable to read from stdin");

            // Try to convert answer to numeric value
            let guess: u16 = match guess
            .trim()
            .parse() {
                Ok(num) => num,
                Err(_) => {
                    println!("{guess} is not a valid number!");
                    // Try again - does not count as incorrect answer
                    continue;
                }
            };

            // Check and see if correct answer
            if problems[prob].check_guess(guess, timer.elapsed()) {            
                println!("Correct! It took you {} seconds to solve.", problems[prob].get_time().as_secs());
                if problems[prob].get_time().as_secs()<=2 {
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
    return save_progress(&problems, &args.config);
}

/// Load question bank from file, and reset if any errors encountered
fn load_progress(problems: &mut Vec<Problem>, path: &Option<PathBuf>) {
    let path = match path {
        Some(path) => path.clone(),
        None => PathBuf::from("math_quiz.ini"),
    };

    // Read entire file into string
    let saved_progress = fs::read_to_string(&path);
    // Replace problems with config file; initialize if any errors encountered
    if saved_progress.is_ok() {        
        println!("Reading progress from {}", path.display());
        let new_problems: Vec<Problem> = serde_json::from_str(&saved_progress.unwrap()).expect("Error deserializing problems");
        *problems=new_problems;
    } else {        
        println!("Unable to open progress file - resetting.");
        // Start with list of problems        
        math_quiz::init_problems(problems);            
    }
}

/// Save progress to specified file or math_quiz.ini if not specified
fn save_progress(problems: &Vec<Problem>, path: &Option<PathBuf>) -> io::Result<()> {    
    let path = match path {
        Some(path) => path.clone(),
        None => PathBuf::from("math_quiz.ini"),
    };
    println!("Saving progress to {}", path.display());
    // Save progress
    return Ok(fs::write(path, serde_json::to_string(&problems).expect("Error serializing problems")).expect("Error saving progress file"));
}