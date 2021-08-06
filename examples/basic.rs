use std::thread::{self, sleep};

use pool::Pool;
use std::time::Duration;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn main() {
    let mut pool = Pool::new();
    pool.pool(1);
    pool.pool(2);
    pool.pool(3);

    let pool_ref = &pool;

    crossbeam::scope(|s| {
        s.spawn(move |_| {
            sleep(ms(1000));
            let v = pool_ref.get();
            println!("t1 got {:?}", v.into_inner());
        });

        s.spawn(move |_| {
            sleep(ms(2000));
            let v = pool_ref.get();
            println!("t2 got {:?}", v.into_inner());
        });
    });
}
