use aether_aggregator::network::hpe_vc::VirtualConnectClient;
use aether_aggregator::network::MidplaneNetworkManager;
use pact_consumer::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_virtual_connect_provision_vlan_contract() {
    let mut pact_builder = PactBuilder::new("Aether-Aggregator", "HPE-OneView");

    // Interaction 1: Login Session Creation
    pact_builder.interaction("login to HPE OneView", "", |mut i| {
        i.request
            .method("POST")
            .path("/rest/login-sessions".to_string())
            .header("content-type", "application/json")
            .header("X-API-Version", "600")
            .json_body(json!({
                "userName": "admin",
                "password": "password",
                "authLoginDomain": "Local"
            }));
        i.response
            .status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "sessionID": "mock-session-token-abc"
            }));
        i
    });

    // Interaction 2: GET Ethernet Networks (filter by VLAN 100, returns empty)
    pact_builder.interaction("get ethernet networks matching VLAN 100", "", |mut i| {
        i.request
            .method("GET")
            .path("/rest/ethernet-networks".to_string())
            .query_param("filter", "vlanId=100")
            .header("auth", "mock-session-token-abc")
            .header("X-API-Version", "600");
        i.response
            .status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "type": "EthernetNetworkCollection",
                "members": [],
                "count": 0,
                "total": 0
            }));
        i
    });

    // Interaction 3: POST Ethernet Network (create network for VLAN 100)
    pact_builder.interaction("create ethernet network for VLAN 100", "", |mut i| {
        i.request
            .method("POST")
            .path("/rest/ethernet-networks".to_string())
            .header("auth", "mock-session-token-abc")
            .header("X-API-Version", "600")
            .header("content-type", "application/json")
            .json_body(json!({
                "name": "Tenant-VLAN-100",
                "vlanId": 100,
                "ethernetNetworkType": "Tagged",
                "type": "ethernet-networkV4"
            }));
        i.response
            .status(201)
            .header("content-type", "application/json")
            .json_body(json!({
                "name": "Tenant-VLAN-100",
                "vlanId": 100,
                "uri": "/rest/ethernet-networks/vlan-100-uuid",
                "ethernetNetworkType": "Tagged",
                "type": "ethernet-networkV4"
            }));
        i
    });

    // Interaction 4: GET Server Profile for slot 3
    pact_builder.interaction("get server profile for slot 3", "", |mut i| {
        i.request
            .method("GET")
            .path("/rest/server-profiles/profile-slot-3".to_string())
            .header("auth", "mock-session-token-abc")
            .header("X-API-Version", "600");
        i.response
            .status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "type": "ServerProfileV12",
                "name": "Blade-Profile-Slot-3",
                "uri": "/rest/server-profiles/profile-slot-3",
                "serverHardwareUri": "/rest/server-hardware/slot-3",
                "connections": [
                    {
                        "id": 1,
                        "name": "FlexNIC-1a",
                        "networkUri": null
                    }
                ]
            }));
        i
    });

    // Interaction 5: PUT Server Profile update (associate network URI)
    pact_builder.interaction(
        "update server profile connection with VLAN 100",
        "",
        |mut i| {
            i.request
                .method("PUT")
                .path("/rest/server-profiles/profile-slot-3".to_string())
                .header("auth", "mock-session-token-abc")
                .header("X-API-Version", "600")
                .header("content-type", "application/json")
                .json_body(json!({
                    "type": "ServerProfileV12",
                    "name": "Blade-Profile-Slot-3",
                    "uri": "/rest/server-profiles/profile-slot-3",
                    "serverHardwareUri": "/rest/server-hardware/slot-3",
                    "connections": [
                        {
                            "id": 1,
                            "name": "FlexNIC-1a",
                            "networkUri": "/rest/ethernet-networks/vlan-100-uuid"
                        }
                    ]
                }));
            i.response
                .status(202)
                .header("content-type", "application/json")
                .json_body(json!({
                    "uri": "/rest/tasks/task-xyz-123",
                    "taskState": "Completed"
                }));
            i
        },
    );

    // Interaction 6: GET Task Status (Completed)
    pact_builder.interaction("get status of profile update task", "", |mut i| {
        i.request
            .method("GET")
            .path("/rest/tasks/task-xyz-123".to_string())
            .header("auth", "mock-session-token-abc")
            .header("X-API-Version", "600");
        i.response
            .status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "uri": "/rest/tasks/task-xyz-123",
                "taskState": "Completed"
            }));
        i
    });

    let mock_server = pact_builder.start_mock_server(None, None);
    let mut base_url = mock_server.url().to_string();
    if base_url.ends_with('/') {
        base_url.pop();
    }

    let mut client =
        VirtualConnectClient::new(base_url, "admin".to_string(), "password".to_string());
    client.poll_interval = std::time::Duration::from_millis(1);

    // Test provision
    let res = client.provision_vlan_interface(3, 100).await;
    assert!(res.is_ok(), "Provisioning VLAN failed: {:?}", res);
}

#[tokio::test]
async fn test_virtual_connect_teardown_vlan_contract() {
    let mut pact_builder = PactBuilder::new("Aether-Aggregator", "HPE-OneView");

    // Interaction 1: Login
    pact_builder.interaction("login for teardown", "", |mut i| {
        i.request
            .method("POST")
            .path("/rest/login-sessions".to_string())
            .header("content-type", "application/json")
            .header("X-API-Version", "600")
            .json_body(json!({
                "userName": "admin",
                "password": "password",
                "authLoginDomain": "Local"
            }));
        i.response
            .status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "sessionID": "mock-session-token-abc"
            }));
        i
    });

    // Interaction 2: GET Server Profile (has networkUri set)
    pact_builder.interaction("get server profile for teardown", "", |mut i| {
        i.request
            .method("GET")
            .path("/rest/server-profiles/profile-slot-3".to_string())
            .header("auth", "mock-session-token-abc")
            .header("X-API-Version", "600");
        i.response
            .status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "type": "ServerProfileV12",
                "name": "Blade-Profile-Slot-3",
                "uri": "/rest/server-profiles/profile-slot-3",
                "serverHardwareUri": "/rest/server-hardware/slot-3",
                "connections": [
                    {
                        "id": 1,
                        "name": "FlexNIC-1a",
                        "networkUri": "/rest/ethernet-networks/vlan-100-uuid"
                    }
                ]
            }));
        i
    });

    // Interaction 3: PUT Server Profile update (cleared networkUri)
    pact_builder.interaction("update server profile to clear network tag", "", |mut i| {
        i.request
            .method("PUT")
            .path("/rest/server-profiles/profile-slot-3".to_string())
            .header("auth", "mock-session-token-abc")
            .header("X-API-Version", "600")
            .header("content-type", "application/json")
            .json_body(json!({
                "type": "ServerProfileV12",
                "name": "Blade-Profile-Slot-3",
                "uri": "/rest/server-profiles/profile-slot-3",
                "serverHardwareUri": "/rest/server-hardware/slot-3",
                "connections": [
                    {
                        "id": 1,
                        "name": "FlexNIC-1a",
                        "networkUri": null
                    }
                ]
            }));
        i.response
            .status(202)
            .header("content-type", "application/json")
            .json_body(json!({
                "uri": "/rest/tasks/task-xyz-999",
                "taskState": "Completed"
            }));
        i
    });

    // Interaction 4: GET Task status for teardown
    pact_builder.interaction("get status of teardown task", "", |mut i| {
        i.request
            .method("GET")
            .path("/rest/tasks/task-xyz-999".to_string())
            .header("auth", "mock-session-token-abc")
            .header("X-API-Version", "600");
        i.response
            .status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "uri": "/rest/tasks/task-xyz-999",
                "taskState": "Completed"
            }));
        i
    });

    let mock_server = pact_builder.start_mock_server(None, None);
    let mut base_url = mock_server.url().to_string();
    if base_url.ends_with('/') {
        base_url.pop();
    }

    let mut client =
        VirtualConnectClient::new(base_url, "admin".to_string(), "password".to_string());
    client.poll_interval = std::time::Duration::from_millis(1);

    // Test teardown
    let res = client.teardown_vlan_interface(3, 100).await;
    assert!(res.is_ok(), "Teardown VLAN failed: {:?}", res);
}

#[tokio::test]
async fn test_virtual_connect_auth_failure() {
    let mut pact_builder = PactBuilder::new("Aether-Aggregator", "HPE-OneView");

    // Login returns 401 Unauthorized
    pact_builder.interaction("failed login to HPE OneView", "", |mut i| {
        i.request
            .method("POST")
            .path("/rest/login-sessions".to_string())
            .header("content-type", "application/json")
            .header("X-API-Version", "600")
            .json_body(json!({
                "userName": "admin",
                "password": "wrong_password",
                "authLoginDomain": "Local"
            }));
        i.response
            .status(401)
            .header("content-type", "application/json")
            .json_body(json!({
                "errorCode": "AUTHENTICATION_FAILED",
                "message": "Invalid username or password."
            }));
        i
    });

    let mock_server = pact_builder.start_mock_server(None, None);
    let mut base_url = mock_server.url().to_string();
    if base_url.ends_with('/') {
        base_url.pop();
    }

    let mut client =
        VirtualConnectClient::new(base_url, "admin".to_string(), "wrong_password".to_string());
    client.poll_interval = std::time::Duration::from_millis(1);

    let res = client.provision_vlan_interface(3, 100).await;
    assert!(res.is_err());
    if let Err(e) = res {
        assert!(
            format!("{:?}", e).contains("Authentication"),
            "Expected Authentication error, got: {:?}",
            e
        );
    }
}

#[tokio::test]
async fn test_virtual_connect_token_refresh_loop() {
    let mut pact_builder = PactBuilder::new("Aether-Aggregator", "HPE-OneView");

    // GET Server Profile with expired token returns 401 Unauthorized
    pact_builder.interaction("get profile with expired token", "", |mut i| {
        i.request
            .method("GET")
            .path("/rest/server-profiles/profile-slot-3".to_string())
            .header("auth", "expired-token")
            .header("X-API-Version", "600");
        i.response
            .status(401)
            .header("content-type", "application/json")
            .json_body(json!({
                "errorCode": "UNAUTHORIZED",
                "message": "Session token invalid or expired"
            }));
        i
    });

    // Login for new token
    pact_builder.interaction("login for new token after 401", "", |mut i| {
        i.request
            .method("POST")
            .path("/rest/login-sessions".to_string())
            .header("content-type", "application/json")
            .header("X-API-Version", "600")
            .json_body(json!({
                "userName": "admin",
                "password": "password",
                "authLoginDomain": "Local"
            }));
        i.response
            .status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "sessionID": "valid-token-456"
            }));
        i
    });

    // GET Server Profile retried with valid-token-456 returns 200 OK
    pact_builder.interaction("get profile with renewed token", "", |mut i| {
        i.request
            .method("GET")
            .path("/rest/server-profiles/profile-slot-3".to_string())
            .header("auth", "valid-token-456")
            .header("X-API-Version", "600");
        i.response
            .status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "type": "ServerProfileV12",
                "name": "Blade-Profile-Slot-3",
                "uri": "/rest/server-profiles/profile-slot-3",
                "serverHardwareUri": "/rest/server-hardware/slot-3",
                "connections": [
                    {
                        "id": 1,
                        "name": "FlexNIC-1a",
                        "networkUri": "/rest/ethernet-networks/vlan-100-uuid"
                    }
                ]
            }));
        i
    });

    // PUT Server Profile to clear network (teardown)
    pact_builder.interaction("teardown profile with renewed token", "", |mut i| {
        i.request
            .method("PUT")
            .path("/rest/server-profiles/profile-slot-3".to_string())
            .header("auth", "valid-token-456")
            .header("X-API-Version", "600")
            .header("content-type", "application/json")
            .json_body(json!({
                "type": "ServerProfileV12",
                "name": "Blade-Profile-Slot-3",
                "uri": "/rest/server-profiles/profile-slot-3",
                "serverHardwareUri": "/rest/server-hardware/slot-3",
                "connections": [
                    {
                        "id": 1,
                        "name": "FlexNIC-1a",
                        "networkUri": null
                    }
                ]
            }));
        i.response
            .status(202)
            .header("content-type", "application/json")
            .json_body(json!({
                "uri": "/rest/tasks/task-xyz-abc",
                "taskState": "Completed"
            }));
        i
    });

    // GET task status
    pact_builder.interaction(
        "get status of teardown task with renewed token",
        "",
        |mut i| {
            i.request
                .method("GET")
                .path("/rest/tasks/task-xyz-abc".to_string())
                .header("auth", "valid-token-456")
                .header("X-API-Version", "600");
            i.response
                .status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "uri": "/rest/tasks/task-xyz-abc",
                    "taskState": "Completed"
                }));
            i
        },
    );

    let mock_server = pact_builder.start_mock_server(None, None);
    let mut base_url = mock_server.url().to_string();
    if base_url.ends_with('/') {
        base_url.pop();
    }

    let mut client =
        VirtualConnectClient::new(base_url, "admin".to_string(), "password".to_string());
    client.poll_interval = std::time::Duration::from_millis(1);
    client
        .preseed_session_token("expired-token".to_string())
        .await;

    // Call teardown (which invokes GET profile, handles 401 retry, then PUT profile)
    let res = client.teardown_vlan_interface(3, 100).await;
    assert!(res.is_ok(), "Teardown with token refresh failed: {:?}", res);
}

#[tokio::test]
async fn test_virtual_connect_profile_not_found() {
    let mut pact_builder = PactBuilder::new("Aether-Aggregator", "HPE-OneView");

    // Login succeeds
    pact_builder.interaction("login for profile not found", "", |mut i| {
        i.request
            .method("POST")
            .path("/rest/login-sessions".to_string())
            .header("content-type", "application/json")
            .header("X-API-Version", "600")
            .json_body(json!({
                "userName": "admin",
                "password": "password",
                "authLoginDomain": "Local"
            }));
        i.response
            .status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "sessionID": "valid-token-123"
            }));
        i
    });

    // GET Ethernet Networks returns existing network URI
    pact_builder.interaction(
        "get existing networks for profile not found test",
        "",
        |mut i| {
            i.request
                .method("GET")
                .path("/rest/ethernet-networks".to_string())
                .query_param("filter", "vlanId=100")
                .header("auth", "valid-token-123")
                .header("X-API-Version", "600");
            i.response
                .status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "type": "EthernetNetworkCollection",
                    "members": [
                        {
                            "name": "Tenant-VLAN-100",
                            "vlanId": 100,
                            "uri": "/rest/ethernet-networks/vlan-100-uuid"
                        }
                    ],
                    "count": 1,
                    "total": 1
                }));
            i
        },
    );

    // GET Server Profile returns 404
    pact_builder.interaction("get non-existent server profile", "", |mut i| {
        i.request
            .method("GET")
            .path("/rest/server-profiles/profile-slot-99".to_string())
            .header("auth", "valid-token-123")
            .header("X-API-Version", "600");
        i.response
            .status(404)
            .header("content-type", "application/json")
            .json_body(json!({
                "errorCode": "RESOURCE_NOT_FOUND",
                "message": "Server profile not found"
            }));
        i
    });

    let mock_server = pact_builder.start_mock_server(None, None);
    let mut base_url = mock_server.url().to_string();
    if base_url.ends_with('/') {
        base_url.pop();
    }

    let mut client =
        VirtualConnectClient::new(base_url, "admin".to_string(), "password".to_string());
    client.poll_interval = std::time::Duration::from_millis(1);

    let res = client.provision_vlan_interface(99, 100).await;
    assert!(res.is_err());
    if let Err(e) = res {
        assert!(
            format!("{:?}", e).contains("NotFound"),
            "Expected NotFound error, got: {:?}",
            e
        );
    }
}

#[tokio::test]
async fn test_virtual_connect_task_failure() {
    let mut pact_builder = PactBuilder::new("Aether-Aggregator", "HPE-OneView");

    // Login succeeds
    pact_builder.interaction("login for task failure", "", |mut i| {
        i.request
            .method("POST")
            .path("/rest/login-sessions".to_string())
            .header("content-type", "application/json")
            .header("X-API-Version", "600")
            .json_body(json!({
                "userName": "admin",
                "password": "password",
                "authLoginDomain": "Local"
            }));
        i.response
            .status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "sessionID": "valid-token-123"
            }));
        i
    });

    // GET Ethernet Networks returns existing network URI
    pact_builder.interaction(
        "get existing networks for task failure test",
        "",
        |mut i| {
            i.request
                .method("GET")
                .path("/rest/ethernet-networks".to_string())
                .query_param("filter", "vlanId=100")
                .header("auth", "valid-token-123")
                .header("X-API-Version", "600");
            i.response
                .status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "type": "EthernetNetworkCollection",
                    "members": [
                        {
                            "name": "Tenant-VLAN-100",
                            "vlanId": 100,
                            "uri": "/rest/ethernet-networks/vlan-100-uuid"
                        }
                    ],
                    "count": 1,
                    "total": 1
                }));
            i
        },
    );

    // GET Server Profile for slot 3
    pact_builder.interaction("get server profile for task failure test", "", |mut i| {
        i.request
            .method("GET")
            .path("/rest/server-profiles/profile-slot-3".to_string())
            .header("auth", "valid-token-123")
            .header("X-API-Version", "600");
        i.response
            .status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "type": "ServerProfileV12",
                "name": "Blade-Profile-Slot-3",
                "uri": "/rest/server-profiles/profile-slot-3",
                "serverHardwareUri": "/rest/server-hardware/slot-3",
                "connections": [
                    {
                        "id": 1,
                        "name": "FlexNIC-1a",
                        "networkUri": null
                    }
                ]
            }));
        i
    });

    // PUT Server Profile returns task-abc with state running/error
    pact_builder.interaction(
        "update server profile connection with task failure",
        "",
        |mut i| {
            i.request
                .method("PUT")
                .path("/rest/server-profiles/profile-slot-3".to_string())
                .header("auth", "valid-token-123")
                .header("X-API-Version", "600")
                .header("content-type", "application/json")
                .json_body(json!({
                    "type": "ServerProfileV12",
                    "name": "Blade-Profile-Slot-3",
                    "uri": "/rest/server-profiles/profile-slot-3",
                    "serverHardwareUri": "/rest/server-hardware/slot-3",
                    "connections": [
                        {
                            "id": 1,
                            "name": "FlexNIC-1a",
                            "networkUri": "/rest/ethernet-networks/vlan-100-uuid"
                        }
                    ]
                }));
            i.response
                .status(202)
                .header("content-type", "application/json")
                .json_body(json!({
                    "uri": "/rest/tasks/task-error-123",
                    "taskState": "Running"
                }));
            i
        },
    );

    // GET Task Status returns "Error"
    pact_builder.interaction(
        "get status of profile update task returning error",
        "",
        |mut i| {
            i.request
                .method("GET")
                .path("/rest/tasks/task-error-123".to_string())
                .header("auth", "valid-token-123")
                .header("X-API-Version", "600");
            i.response
                .status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "uri": "/rest/tasks/task-error-123",
                    "taskState": "Error"
                }));
            i
        },
    );

    let mock_server = pact_builder.start_mock_server(None, None);
    let mut base_url = mock_server.url().to_string();
    if base_url.ends_with('/') {
        base_url.pop();
    }

    let mut client =
        VirtualConnectClient::new(base_url, "admin".to_string(), "password".to_string());
    client.poll_interval = std::time::Duration::from_millis(1);

    let res = client.provision_vlan_interface(3, 100).await;
    assert!(res.is_err());
    if let Err(e) = res {
        assert!(
            format!("{:?}", e).contains("Other"),
            "Expected Other error (task failure), got: {:?}",
            e
        );
    }
}
