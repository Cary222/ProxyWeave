//! Marketplace discovery, online catalog retrieval, and categorized indexing.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketplacePluginItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub repo_url: String,
    pub kind: String,     // "managed" | "external"
    pub category: String, // "proxy" | "protocol" | "local_model" | "tools"
    pub tags: Vec<String>,
    pub protocol: String, // "openai_chat" | "openai_responses" | "anthropic_messages" | "multi"
    pub stars: Option<u32>,
    pub download_url: Option<String>,
    pub release_tag: Option<String>,
    pub icon: Option<String>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketplaceCatalogResponse {
    pub items: Vec<MarketplacePluginItem>,
    pub is_offline: bool,
    pub notice: Option<String>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
struct GithubSearchResponse {
    #[serde(default)]
    items: Vec<GithubRepoItem>,
}

#[derive(Debug, Deserialize)]
struct GithubRepoItem {
    name: String,
    owner: Option<GithubOwner>,
    html_url: String,
    description: Option<String>,
    stargazers_count: Option<u32>,
    topics: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GithubOwner {
    login: String,
    avatar_url: Option<String>,
}

/// Returns the built-in catalog of recommended plugins for instant offline discovery.
pub fn get_builtin_catalog() -> Vec<MarketplacePluginItem> {
    vec![
        MarketplacePluginItem {
            id: "wb2api".to_string(),
            name: "WorkBuddy 2API".to_string(),
            description: "WorkBuddy 平台代理转换网关，提供标准 OpenAI 接口兼容与请求转换"
                .to_string(),
            version: "1.13.0".to_string(),
            author: "WorkBuddy Team".to_string(),
            repo_url: "https://github.com/router-for-me/workbuddy2api".to_string(),
            kind: "managed".to_string(),
            category: "proxy".to_string(),
            tags: vec![
                "workbuddy".to_string(),
                "2api".to_string(),
                "openai".to_string(),
                "managed".to_string(),
            ],
            protocol: "openai_chat".to_string(),
            stars: None,
            download_url: None,
            release_tag: Some("v1.13.0".to_string()),
            icon: None,
            verified: false,
        },
        MarketplacePluginItem {
            id: "external-cpa".to_string(),
            name: "CLIProxyAPI".to_string(),
            description: "高性能全协议反向代理与中继插件，支持多协议转换与管理面板挂载".to_string(),
            version: "7.2.115".to_string(),
            author: "router-for-me".to_string(),
            repo_url: "https://github.com/router-for-me/CLIProxyAPI".to_string(),
            kind: "external".to_string(),
            category: "proxy".to_string(),
            tags: vec![
                "cliproxyapi".to_string(),
                "cpa".to_string(),
                "multi-protocol".to_string(),
                "external".to_string(),
            ],
            protocol: "multi".to_string(),
            stars: None,
            download_url: None,
            release_tag: Some("v7.2.115".to_string()),
            icon: None,
            verified: false,
        },
        MarketplacePluginItem {
            id: "ollama-bridge".to_string(),
            name: "Ollama 兼容网桥".to_string(),
            description: "本地 Ollama 实例连接网桥，无缝将本地大模型映射为标准 OpenAI API"
                .to_string(),
            version: "0.4.1".to_string(),
            author: "ProxyWeave Community".to_string(),
            repo_url: "https://github.com/proxyweave/ollama-bridge".to_string(),
            kind: "managed".to_string(),
            category: "local_model".to_string(),
            tags: vec![
                "ollama".to_string(),
                "local".to_string(),
                "bridge".to_string(),
            ],
            protocol: "openai_chat".to_string(),
            stars: None,
            download_url: None,
            release_tag: Some("v0.4.1".to_string()),
            icon: None,
            verified: false,
        },
        MarketplacePluginItem {
            id: "vllm-external".to_string(),
            name: "vLLM 本地高并发服务挂载".to_string(),
            description: "局域网或本机高性能 vLLM 推理实例挂载，支持 PagedAttention 吞吐优化"
                .to_string(),
            version: "0.6.2".to_string(),
            author: "vLLM Team".to_string(),
            repo_url: "https://github.com/vllm-project/vllm".to_string(),
            kind: "external".to_string(),
            category: "local_model".to_string(),
            tags: vec![
                "vllm".to_string(),
                "gpu".to_string(),
                "high-concurrency".to_string(),
            ],
            protocol: "openai_chat".to_string(),
            stars: None,
            download_url: None,
            release_tag: Some("v0.6.2".to_string()),
            icon: None,
            verified: false,
        },
        MarketplacePluginItem {
            id: "deepseek-harness".to_string(),
            name: "DeepSeek 本地专属服务挂载".to_string(),
            description: "DeepSeek 本地专属服务挂载，支持 R1/V3 推理加速与上下文管理".to_string(),
            version: "1.2.0".to_string(),
            author: "DeepSeek Harness Team".to_string(),
            repo_url: "https://github.com/proxyweave/deepseek-harness".to_string(),
            kind: "managed".to_string(),
            category: "proxy".to_string(),
            tags: vec![
                "deepseek".to_string(),
                "reasoning".to_string(),
                "managed".to_string(),
            ],
            protocol: "openai_chat".to_string(),
            stars: None,
            download_url: None,
            release_tag: Some("v1.2.0".to_string()),
            icon: None,
            verified: false,
        },
        MarketplacePluginItem {
            id: "open-webui-proxy".to_string(),
            name: "Open WebUI 桥接插件".to_string(),
            description: "Open WebUI 桥接插件，实现现代化 Web 界面与 ProxyWeave 网关打通"
                .to_string(),
            version: "0.3.8".to_string(),
            author: "open-webui".to_string(),
            repo_url: "https://github.com/open-webui/open-webui".to_string(),
            kind: "external".to_string(),
            category: "tools".to_string(),
            tags: vec!["webui".to_string(), "chat".to_string(), "tools".to_string()],
            protocol: "multi".to_string(),
            stars: None,
            download_url: None,
            release_tag: Some("v0.3.8".to_string()),
            icon: None,
            verified: false,
        },
    ]
}

const GITHUB_SEARCH_URL: &str =
    "https://api.github.com/search/repositories?q=topic:proxyweave-plugin&sort=stars&order=desc";

fn marketplace_request_error(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "连接 GitHub 超时，请检查网络或代理后重试".to_string()
    } else if error.is_connect() {
        "无法建立 GitHub 连接，请检查网络、代理或系统证书".to_string()
    } else {
        "GitHub 请求失败，请检查网络后重试".to_string()
    }
}

async fn fetch_github_search(
    client: &reqwest::Client,
    url: &str,
) -> Result<GithubSearchResponse, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| marketplace_request_error(&error))?;
    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            403 | 429 => format!(
                "GitHub 拒绝请求或触发访问限额 (HTTP {}), 请稍后重试",
                status.as_u16()
            ),
            _ => format!("GitHub API 返回 HTTP {}", status.as_u16()),
        });
    }
    response
        .json()
        .await
        .map_err(|_| "GitHub 返回的数据格式无效，请稍后重试".to_string())
}

async fn fetch_online_marketplace() -> Result<Vec<MarketplacePluginItem>, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("proxyweave-desktop")
        .build()
        .map_err(|_| "无法初始化 GitHub HTTP 客户端".to_string())?;
    let payload = fetch_github_search(&client, GITHUB_SEARCH_URL).await?;

    let mut items = Vec::new();
    for repo in payload.items {
        let id = repo.name.to_lowercase();
        let name = repo.name;
        let description = repo
            .description
            .unwrap_or_else(|| "GitHub 社区插件".to_string());
        let author = repo
            .owner
            .as_ref()
            .map(|o| o.login.clone())
            .unwrap_or_else(|| "community".to_string());
        let repo_url = repo.html_url;
        let tags = repo.topics.unwrap_or_default();

        let kind = if tags.iter().any(|t| t == "external") {
            "external".to_string()
        } else {
            "managed".to_string()
        };

        let category = if tags
            .iter()
            .any(|t| t == "local_model" || t == "local-model" || t == "ollama" || t == "vllm")
        {
            "local_model".to_string()
        } else if tags
            .iter()
            .any(|t| t == "tools" || t == "tool" || t == "webui")
        {
            "tools".to_string()
        } else if tags.iter().any(|t| t == "protocol") {
            "protocol".to_string()
        } else {
            "proxy".to_string()
        };

        let protocol = if tags
            .iter()
            .any(|t| t == "anthropic" || t == "anthropic_messages")
        {
            "anthropic_messages".to_string()
        } else if tags.iter().any(|t| t == "multi") {
            "multi".to_string()
        } else if tags.iter().any(|t| t == "openai_responses") {
            "openai_responses".to_string()
        } else {
            "openai_chat".to_string()
        };

        let icon = repo.owner.and_then(|o| o.avatar_url);
        let verified = false;

        items.push(MarketplacePluginItem {
            id,
            name,
            description,
            version: "0.1.0".to_string(),
            author,
            repo_url,
            kind,
            category,
            tags,
            protocol,
            stars: repo.stargazers_count,
            download_url: None,
            release_tag: None,
            icon,
            verified,
        });
    }

    Ok(items)
}

/// Searches the marketplace catalog by query and category, optionally refreshing from GitHub.
pub async fn search_marketplace(
    query: Option<String>,
    category: Option<String>,
    refresh_online: bool,
) -> Result<MarketplaceCatalogResponse, String> {
    let (all_items, is_offline, notice) = if refresh_online {
        match fetch_online_marketplace().await {
            Ok(online_items) => {
                let mut merged = get_builtin_catalog();
                for online_item in online_items {
                    if let Some(existing) = merged.iter_mut().find(|b| {
                        b.id == online_item.id
                            || b.repo_url.eq_ignore_ascii_case(&online_item.repo_url)
                    }) {
                        if online_item.stars.is_some() {
                            existing.stars = online_item.stars;
                        }
                        if existing.icon.is_none() && online_item.icon.is_some() {
                            existing.icon = online_item.icon;
                        }
                    } else {
                        merged.push(online_item);
                    }
                }
                (merged, false, None)
            }
            Err(e) => (
                get_builtin_catalog(),
                true,
                Some(format!("无法连接 GitHub 市场，已切换为内置生态清单: {e}")),
            ),
        }
    } else {
        (
            get_builtin_catalog(),
            true,
            Some("已加载内置生态推荐清单 (离线模式)".to_string()),
        )
    };

    let query_trim = query.unwrap_or_default().trim().to_lowercase();
    let cat_trim = category.unwrap_or_default().trim().to_lowercase();

    let filtered: Vec<MarketplacePluginItem> = all_items
        .into_iter()
        .filter(|item| {
            if !cat_trim.is_empty() && cat_trim != "all" && cat_trim != "全部" {
                let matches_kind = (cat_trim == "managed" || cat_trim == "external")
                    && item.kind.eq_ignore_ascii_case(&cat_trim);
                let matches_cat = item.category.eq_ignore_ascii_case(&cat_trim);
                if !matches_kind && !matches_cat {
                    return false;
                }
            }

            if !query_trim.is_empty() {
                let id_match = item.id.to_lowercase().contains(&query_trim);
                let name_match = item.name.to_lowercase().contains(&query_trim);
                let desc_match = item.description.to_lowercase().contains(&query_trim);
                let author_match = item.author.to_lowercase().contains(&query_trim);
                let tag_match = item
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&query_trim));
                if !id_match && !name_match && !desc_match && !author_match && !tag_match {
                    return false;
                }
            }

            true
        })
        .collect();

    let total = filtered.len();
    Ok(MarketplaceCatalogResponse {
        items: filtered,
        is_offline,
        notice,
        total,
    })
}

/// Retrieves the marketplace catalog without query/category filters.
pub async fn get_marketplace_catalog(
    refresh_online: bool,
) -> Result<MarketplaceCatalogResponse, String> {
    search_marketplace(None, None, refresh_online).await
}

#[cfg(test)]
mod network_tests {
    use super::*;
    use std::io::{Read, Write};

    fn endpoint(status: u16, body: &str, delay: std::time::Duration) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let body = body.to_owned();
        std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut request = [0u8; 4096];
            let _ = socket.read(&mut request);
            std::thread::sleep(delay);
            let response = format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            let _ = socket.write_all(response.as_bytes());
        });
        format!("http://{address}/?token=owned-secret-marker")
    }

    #[tokio::test]
    async fn github_http_failures_and_json_errors_are_distinct_and_redacted() {
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        for status in [403, 429, 500] {
            let url = endpoint(status, "owned-private-body", std::time::Duration::ZERO);
            let error = fetch_github_search(&client, &url).await.unwrap_err();
            assert!(error.contains(&status.to_string()));
            assert!(!error.contains("owned-") && !error.contains("127.0.0.1"));
            assert_eq!(error.contains("访问限额"), status == 403 || status == 429);
        }
        let url = endpoint(200, "owned-private-invalid-json", std::time::Duration::ZERO);
        let error = fetch_github_search(&client, &url).await.unwrap_err();
        assert_eq!(error, "GitHub 返回的数据格式无效，请稍后重试");
        let url = endpoint(200, r#"{"items":[]}"#, std::time::Duration::ZERO);
        assert!(fetch_github_search(&client, &url)
            .await
            .unwrap()
            .items
            .is_empty());
    }

    #[tokio::test]
    async fn github_timeout_and_connection_errors_do_not_expose_request_url() {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_millis(50))
            .build()
            .unwrap();
        let url = endpoint(
            200,
            r#"{"items":[]}"#,
            std::time::Duration::from_millis(300),
        );
        let error = fetch_github_search(&client, &url).await.unwrap_err();
        assert_eq!(error, "连接 GitHub 超时，请检查网络或代理后重试");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        // Windows connection refusal can take longer than the delayed-response test budget.
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let error = fetch_github_search(
            &client,
            &format!("http://{address}/?token=owned-secret-marker"),
        )
        .await
        .unwrap_err();
        assert_eq!(error, "无法建立 GitHub 连接，请检查网络、代理或系统证书");
    }
}
