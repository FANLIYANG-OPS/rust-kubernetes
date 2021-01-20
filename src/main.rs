fn main() {
    let m = (1..5).fold(1u64, |mul, x| {
        println!("mul is {}", mul);
        println!("x is {}", x);
        mul + x
    });
    println!("Hello, world!{}", m);
}
