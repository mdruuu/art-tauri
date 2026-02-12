use crate::art_api::fetch_random_artwork;
use crate::Artwork;
use reqwest::Client;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

const CACHE_SIZE: usize = 5;

pub struct ArtCache {
    client: Client,
    cache: Arc<Mutex<VecDeque<Artwork>>>,
    history: Arc<Mutex<Vec<Artwork>>>,
    history_index: Arc<Mutex<Option<usize>>>,
}

impl ArtCache {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("ArtDisplay/0.1 (Desktop Art Viewer)")
                .build()
                .unwrap_or_default(),
            cache: Arc::new(Mutex::new(VecDeque::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            history_index: Arc::new(Mutex::new(None)),
        }
    }

    /// Start background prefetch loop
    pub fn start_prefetch(&self) {
        let client = self.client.clone();
        let cache = self.cache.clone();

        tauri::async_runtime::spawn(async move {
            loop {
                let current_len = cache.lock().await.len();
                if current_len < CACHE_SIZE {
                    match fetch_random_artwork(&client).await {
                        Ok(artwork) => {
                            let mut c = cache.lock().await;
                            if c.len() < CACHE_SIZE {
                                log::info!(
                                    "Cached artwork: {} (cache size: {})",
                                    artwork.title,
                                    c.len() + 1
                                );
                                c.push_back(artwork);
                            }
                        }
                        Err(e) => {
                            log::error!("Prefetch failed: {e}");
                        }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        });
    }

    /// Get the next artwork (from cache or fetch live)
    pub async fn next(&self) -> Result<Artwork, String> {
        // If browsing history, move forward
        {
            let mut idx = self.history_index.lock().await;
            if let Some(i) = *idx {
                let history = self.history.lock().await;
                if i + 1 < history.len() {
                    *idx = Some(i + 1);
                    return Ok(history[i + 1].clone());
                } else {
                    // At end of history, fall through to get new artwork
                    *idx = None;
                }
            }
        }

        // Try cache first, then fetch live
        let artwork = {
            let mut cache = self.cache.lock().await;
            if let Some(art) = cache.pop_front() {
                art
            } else {
                drop(cache);
                fetch_random_artwork(&self.client).await?
            }
        };

        // Add to history
        {
            let mut history = self.history.lock().await;
            history.push(artwork.clone());
            // Keep history reasonable
            if history.len() > 50 {
                history.drain(0..25);
            }
        }

        Ok(artwork)
    }

    /// Go back in history
    pub async fn prev(&self) -> Result<Artwork, String> {
        let mut idx = self.history_index.lock().await;
        let history = self.history.lock().await;

        if history.is_empty() {
            return Err("No history".into());
        }

        let new_idx = match *idx {
            Some(0) => return Err("At beginning of history".into()),
            Some(i) => i - 1,
            None => {
                // Start browsing from the last item
                if history.len() >= 2 {
                    history.len() - 2
                } else {
                    return Err("No previous artwork".into());
                }
            }
        };

        *idx = Some(new_idx);
        Ok(history[new_idx].clone())
    }

    /// Get current artwork without advancing
    pub async fn current(&self) -> Option<Artwork> {
        let idx = self.history_index.lock().await;
        let history = self.history.lock().await;

        match *idx {
            Some(i) => history.get(i).cloned(),
            None => history.last().cloned(),
        }
    }
}
