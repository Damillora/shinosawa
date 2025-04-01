
#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    use crate::printk;

    printk!("running {} tests", tests.len());
    for test in tests {
        test();
    }
}