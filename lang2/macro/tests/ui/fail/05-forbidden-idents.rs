use ink_lang2 as ink;

#[ink::contract(version = "0.1.0")]
mod noop {
    #[ink(storage)]
    struct Noop {}

    impl Noop {
        #[ink(constructor)]
        fn new(&mut self) {}

        #[ink(message)]
        fn __ink_noop(&self) {}
    }
}

fn main() {}
