use duva::domains::cluster_actors::heartbeats::scheduler::LEADER_HEARTBEAT_INTERVAL_MAX;

use crate::common::{Client, ServerEnv, check_internodes_communication, spawn_server_process};

fn run_cluster_forget_makes_all_nodes_forget_target_node(
    with_append_only: bool,
) -> anyhow::Result<()> {
    // GIVEN
    const HOP_COUNT: usize = 0;

    let env = ServerEnv::default().with_ttl(500).with_hf(2).with_append_only(with_append_only);
    let mut leader_p = spawn_server_process(&env, true)?;

    let repl_env = ServerEnv::default()
        .with_leader_bind_addr(leader_p.bind_addr().clone())
        .with_hf(10)
        .with_append_only(with_append_only);
    let mut repl_p = spawn_server_process(&repl_env, true)?;

    let repl_env2 = ServerEnv::default()
        .with_leader_bind_addr(leader_p.bind_addr().clone())
        .with_hf(10)
        .with_append_only(with_append_only);
    let mut repl_p2 = spawn_server_process(&repl_env2, true)?;

    check_internodes_communication(
        &mut [&mut leader_p, &mut repl_p, &mut repl_p2],
        HOP_COUNT,
        1000,
    )?;

    // WHEN
    let mut client_handler = Client::new(leader_p.port);
    let replica_id = repl_p.bind_addr();
    let response1 = client_handler.send_and_get(format!("cluster forget {}", &replica_id), 1);
    assert_eq!(response1, vec!["OK"]);

    // THEN
    let response2 = client_handler.send_and_get("cluster info", 1);
    assert_eq!(response2.first().unwrap(), "cluster_known_nodes:1");

    let mut repl_cli = Client::new(repl_p2.port);

    std::thread::sleep(std::time::Duration::from_millis(LEADER_HEARTBEAT_INTERVAL_MAX + 1));

    let response2 = repl_cli.send_and_get("cluster info", 1);
    assert_eq!(response2.first().unwrap(), "cluster_known_nodes:1");

    Ok(())
}

#[test]
fn test_cluster_forget_makes_all_nodes_forget_target_node() -> anyhow::Result<()> {
    run_cluster_forget_makes_all_nodes_forget_target_node(false)?;
    run_cluster_forget_makes_all_nodes_forget_target_node(true)?;

    Ok(())
}
