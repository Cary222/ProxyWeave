use proxyweave_plugin::marketplace::{
    get_builtin_catalog, get_marketplace_catalog, search_marketplace,
};

#[test]
fn test_builtin_catalog_loaded_and_verified() {
    let items = get_builtin_catalog();
    assert!(
        items.len() >= 6,
        "Built-in catalog must contain at least 6 items, found {}",
        items.len()
    );

    // Verify key plugins exist
    let ids: Vec<&str> = items.iter().map(|item| item.id.as_str()).collect();
    assert!(ids.contains(&"wb2api"), "wb2api must be in catalog");
    assert!(
        ids.contains(&"external-cpa"),
        "external-cpa must be in catalog"
    );
    assert!(
        ids.contains(&"ollama-bridge"),
        "ollama-bridge must be in catalog"
    );
    assert!(
        ids.contains(&"vllm-external"),
        "vllm-external must be in catalog"
    );
    assert!(
        ids.contains(&"deepseek-harness"),
        "deepseek-harness must be in catalog"
    );
    assert!(
        ids.contains(&"open-webui-proxy"),
        "open-webui-proxy must be in catalog"
    );

    // Verify all built-in items are honest (no fake stars or fake verification) and have valid protocols
    for item in &items {
        assert!(
            !item.verified,
            "Built-in item {} should not be marked verified without audit",
            item.id
        );
        assert!(
            item.stars.is_none(),
            "Built-in item {} should not have fake star ratings",
            item.id
        );
        assert!(
            !item.name.is_empty(),
            "Item {} must have non-empty name",
            item.id
        );
        assert!(!item.author.is_empty(), "Item {} must have author", item.id);
        assert!(
            item.kind == "managed" || item.kind == "external",
            "Item {} kind must be managed or external, got {}",
            item.id,
            item.kind
        );
        assert!(
            [
                "openai_chat",
                "openai_responses",
                "anthropic_messages",
                "multi"
            ]
            .contains(&item.protocol.as_str()),
            "Item {} protocol {} is not recognized",
            item.id,
            item.protocol
        );
    }
}

#[tokio::test]
async fn test_search_by_query_name_tag_description() {
    // 1. Search by name / keyword "WorkBuddy"
    let res = search_marketplace(Some("WorkBuddy".to_string()), None, false)
        .await
        .expect("search should succeed");
    assert!(!res.items.is_empty());
    assert!(res.items.iter().any(|i| i.id == "wb2api"));

    // 2. Search by tag "openai"
    let res_tag = search_marketplace(Some("openai".to_string()), None, false)
        .await
        .expect("tag search should succeed");
    assert!(!res_tag.items.is_empty());
    assert!(res_tag.items.iter().any(|i| i.id == "wb2api"));

    // 3. Search by description keyword "反向代理"
    let res_desc = search_marketplace(Some("反向代理".to_string()), None, false)
        .await
        .expect("desc search should succeed");
    assert!(!res_desc.items.is_empty());
    assert!(res_desc.items.iter().any(|i| i.id == "external-cpa"));

    // 4. Search by author "DeepSeek"
    let res_author = search_marketplace(Some("DeepSeek".to_string()), None, false)
        .await
        .expect("author search should succeed");
    assert!(!res_author.items.is_empty());
    assert!(res_author.items.iter().any(|i| i.id == "deepseek-harness"));

    // 5. Non-matching query returns empty
    let res_empty = search_marketplace(Some("non_existent_plugin_xyz".to_string()), None, false)
        .await
        .expect("empty search should succeed");
    assert_eq!(res_empty.items.len(), 0);
    assert_eq!(res_empty.total, 0);
}

#[tokio::test]
async fn test_filter_by_category_and_kind() {
    // 1. Filter by kind "managed"
    let res_managed = search_marketplace(None, Some("managed".to_string()), false)
        .await
        .expect("managed filter should succeed");
    assert!(!res_managed.items.is_empty());
    for item in &res_managed.items {
        assert_eq!(item.kind, "managed");
    }

    // 2. Filter by kind "external"
    let res_external = search_marketplace(None, Some("external".to_string()), false)
        .await
        .expect("external filter should succeed");
    assert!(!res_external.items.is_empty());
    for item in &res_external.items {
        assert_eq!(item.kind, "external");
    }

    // 3. Filter by category "local_model"
    let res_local = search_marketplace(None, Some("local_model".to_string()), false)
        .await
        .expect("local_model filter should succeed");
    assert!(!res_local.items.is_empty());
    for item in &res_local.items {
        assert_eq!(item.category, "local_model");
    }
    let local_ids: Vec<&str> = res_local.items.iter().map(|i| i.id.as_str()).collect();
    assert!(local_ids.contains(&"ollama-bridge"));
    assert!(local_ids.contains(&"vllm-external"));

    // 4. Filter by category "proxy"
    let res_proxy = search_marketplace(None, Some("proxy".to_string()), false)
        .await
        .expect("proxy filter should succeed");
    assert!(!res_proxy.items.is_empty());
    for item in &res_proxy.items {
        assert_eq!(item.category, "proxy");
    }

    // 5. Filter by category "tools"
    let res_tools = search_marketplace(None, Some("tools".to_string()), false)
        .await
        .expect("tools filter should succeed");
    assert!(!res_tools.items.is_empty());
    assert!(res_tools.items.iter().any(|i| i.id == "open-webui-proxy"));
}

#[tokio::test]
async fn test_offline_fallback_and_degradation() {
    // Calling with refresh_online: false returns offline status
    let res = get_marketplace_catalog(false)
        .await
        .expect("catalog should succeed");
    assert!(res.is_offline);
    assert!(res.notice.is_some());
    assert!(res.items.len() >= 6);
    assert_eq!(res.total, res.items.len());

    // Calling with refresh_online: true should gracefully succeed even if network fails
    let online_res = get_marketplace_catalog(true)
        .await
        .expect("online call should gracefully degrade and never return Err");
    assert!(online_res.items.len() >= 6);
    assert_eq!(online_res.total, online_res.items.len());
}
