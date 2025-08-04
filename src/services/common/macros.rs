/// Creates a watch method that combines multiple stream sources into one.
///
/// This macro provides a consistent watch API for types that need to aggregate
/// multiple change streams. It takes any fields that have a `watch()` method
/// and combines them into a single stream that emits the full struct whenever
/// any field changes.
///
/// # Example
/// ```ignore
/// impl MyStruct {
///     pub fn watch(&self) -> impl Stream<Item = Self> + Send {
///         watch_all!(self, field1, field2, field3)
///     }
/// }
/// ```
#[macro_export]
macro_rules! watch_all {
    ($self:expr, $($source:ident),+ $(,)?) => {
        {
            use ::futures::StreamExt;

            let cloned = $self.clone();
            let streams: Vec<::futures::stream::BoxStream<'_, ()>> = vec![
                $($self.$source.watch().map(|_| ()).boxed(),)+
            ];
            ::futures::stream::select_all(streams).map(move |_| cloned.clone())
        }
    };
}
