use ink_lang2 as ink;

#[ink::contract(version = "0.1.0")]
mod noop {
    #[ink(storage)]
    struct Noop {}

    impl Noop {
        #[ink(constructor)]
        unsafe fn invalid_return(&mut self) {}

        #[ink(message)]
        fn noop(&self) {}
    }
}

fn main() {}
