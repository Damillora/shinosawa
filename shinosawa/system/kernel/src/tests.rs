use crate::printk;

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {

    printk!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
}