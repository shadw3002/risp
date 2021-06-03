use std::io::{self, Write, Read};

fn main() {
    let mut interpreter = risp::Interpreter::new();
    let mut parser = risp::Parser::new();

    loop {
        print!("> ");
        io::stdout().flush().ok().expect("flush stdout");

        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Err(err) => {
                println!("failed to read: {}", err);
                break;
            }
            Ok(0) => {
                println!("exit");
                break;
            },
            Ok(_) => {
                parse(line, &mut interpreter, &mut parser);
            },

        }
    }
}


fn parse(input: String, interpreter: &mut risp::Interpreter, parser: &mut risp::Parser) {
    match parser.parse(input.as_bytes()) {
        Ok(exps) => for (i, exp) in exps.iter().enumerate() {
            println!("${} exp: {:?}", i, exp);
        },
        Err(err) => println!("failed to parse: {}", 1),
    }
}
