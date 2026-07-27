use wpress_oxide::Reader;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: wpress_ls <file.wpress> <out_dir>");
        return;
    }
    
    let path = Path::new(&args[1]);
    let out_dir = Path::new(&args[2]);
    let mut reader = match Reader::new(path) {
        Ok(r) => r,
        Err(e) => {
            println!("Error opening wpress: {}", e);
            return;
        }
    };
    
    match reader.extract_to(out_dir) {
        Ok(_) => println!("Successfully extracted to {}", out_dir.display()),
        Err(e) => println!("Error extracting: {}", e),
    }
}
