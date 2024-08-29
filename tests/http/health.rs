use crate::suite::TestApp;
use std::collections::HashMap;

#[actix_web::test]
async fn health_check_works() {
    // Arrange
    let application = TestApp::spawn(HashMap::new()).await;
    // Act
    let response = application.get("health_check").await;
    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(48), response.content_length());
}
