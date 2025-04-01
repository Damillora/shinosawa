use crate::printk;

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {

    printk!("running {} tests", tests.len());
    for test in tests {
        test();
    }
}