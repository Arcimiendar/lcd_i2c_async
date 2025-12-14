pub trait I2CAsync {
    type Error: core::fmt::Debug;

    fn write_async(
        &mut self,
        addr: impl Into<u16>,
        bytes: impl IntoIterator<Item = u8>,
    ) -> impl core::future::Future<Output = Result<(), Self::Error>> + Send;
}

pub trait SleepableAsync {
    fn sleep_for_millis(&mut self, millis: u64) -> impl core::future::Future<Output = ()> + Send;
    fn sleep_for_micros(&mut self, millis: u64) -> impl core::future::Future<Output = ()> + Send;
}
