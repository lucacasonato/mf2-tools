#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
  let _ = mf2_parser::parse(data);
});
