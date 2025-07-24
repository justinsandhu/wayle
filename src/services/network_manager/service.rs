use zbus::Connection;

use crate::services::common::Property;

pub struct NetworkService {
    connection: Connection,
    network: Property<String>,
}
