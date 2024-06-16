use mf2_parser::parse;

fn main() {
  let message = std::fs::read_to_string("./test.mf2").unwrap();
  let msg = parse(&message);
  println!("{msg:#?}")
}
