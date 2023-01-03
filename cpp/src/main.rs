use bstr::BStr;
use initial::lines::Lines;
use preprocessor::lexer::lex;

fn main() {
    let contents = std::fs::read("main.c").unwrap();
    let src = Lines::new(BStr::new(&contents))
        .merge_escaped_newlines()
        .delete_comments()
        .finish();
    for token in lex(src.as_ref()) {
        println!("{token}");
    }
}
