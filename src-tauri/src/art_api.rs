use crate::Artwork;
use rand::Rng;
use reqwest::Client;
use serde::Deserialize;

/// Strip HTML tags from a string
fn strip_html(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result
}

/// Download image bytes with validation: checks HTTP status, content-type, and minimum size.
/// Returns (bytes, mime_type) or None on any failure.
async fn download_image(client: &Client, url: &str) -> Option<(Vec<u8>, String)> {
    let mut req = client.get(url);
    // AIC's IIIF server requires a Referer header
    if url.contains("artic.edu") {
        req = req.header("Referer", "https://www.artic.edu/");
    }
    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        log::warn!("Image HTTP {}: {}", resp.status(), url);
        return None;
    }
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !content_type.starts_with("image/") {
        log::warn!("Not an image ({}): {}", content_type, url);
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.len() < 1000 {
        log::warn!("Image too small ({} bytes): {}", bytes.len(), url);
        return None;
    }
    // Extract just the mime type (strip charset etc.)
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or("image/jpeg")
        .trim()
        .to_string();
    Some((bytes.to_vec(), mime))
}

// ── Met Museum API ──

#[derive(Deserialize)]
struct MetSearchResult {
    #[serde(rename = "objectIDs")]
    object_ids: Option<Vec<u64>>,
}

#[derive(Deserialize)]
struct MetObject {
    #[serde(rename = "objectID")]
    object_id: u64,
    title: Option<String>,
    #[serde(rename = "artistDisplayName")]
    artist_display_name: Option<String>,
    #[serde(rename = "objectDate")]
    object_date: Option<String>,
    medium: Option<String>,
    #[serde(rename = "primaryImage")]
    primary_image: Option<String>,
}

pub async fn fetch_met_artwork(client: &Client) -> Result<Artwork, String> {
    let search_terms = [
        "painting", "landscape", "portrait", "still life", "sculpture",
        "impressionism", "renaissance", "abstract", "nature", "mythology",
    ];
    let term = search_terms[rand::rng().random_range(0..search_terms.len())];

    let search: MetSearchResult = client
        .get("https://collectionapi.metmuseum.org/public/collection/v1/search")
        .query(&[("hasImages", "true"), ("q", term)])
        .send()
        .await
        .map_err(|e| format!("Met search failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Met search parse failed: {e}"))?;

    let ids = search.object_ids.ok_or("No results from Met")?;
    if ids.is_empty() {
        return Err("Empty Met results".into());
    }

    // Try up to 5 random objects to find one with an image
    for _ in 0..5 {
        let id = ids[rand::rng().random_range(0..ids.len())];
        let url = format!(
            "https://collectionapi.metmuseum.org/public/collection/v1/objects/{id}"
        );

        let obj: MetObject = match client.get(&url).send().await {
            Ok(resp) => match resp.json().await {
                Ok(o) => o,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        let image_url = match &obj.primary_image {
            Some(url) if !url.is_empty() => url.clone(),
            _ => continue,
        };

        let (image_bytes, mime) = match download_image(client, &image_url).await {
            Some(result) => result,
            None => continue, // try next random object
        };

        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &image_bytes,
        );

        return Ok(Artwork {
            id: format!("met-{}", obj.object_id),
            title: strip_html(&obj.title.unwrap_or_else(|| "Untitled".into())),
            artist: obj
                .artist_display_name
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "Unknown Artist".into()),
            date: obj.object_date.unwrap_or_default(),
            medium: obj.medium.unwrap_or_default(),
            source: "The Metropolitan Museum of Art".into(),
            image_base64: format!("data:{mime};base64,{b64}"),
        });
    }

    Err("Could not find Met artwork with image".into())
}

// ── Art Institute of Chicago API ──

#[derive(Deserialize)]
struct AicSearchResponse {
    #[serde(default)]
    data: Vec<AicArtwork>,
    #[serde(default)]
    config: AicConfig,
}

#[derive(Deserialize, Default)]
struct AicConfig {
    #[serde(default = "default_iiif_url")]
    iiif_url: String,
}

fn default_iiif_url() -> String {
    "https://www.artic.edu/iiif/2".to_string()
}

#[derive(Deserialize)]
struct AicArtwork {
    id: u64,
    title: Option<String>,
    artist_display: Option<String>,
    date_display: Option<String>,
    medium_display: Option<String>,
    image_id: Option<String>,
}

pub async fn fetch_aic_artwork(client: &Client) -> Result<Artwork, String> {
    let search_terms = [
        "painting", "landscape", "impressionist", "modern", "watercolor",
        "oil", "portrait", "nature", "classical", "abstract",
    ];
    let term = search_terms[rand::rng().random_range(0..search_terms.len())];
    let page = rand::rng().random_range(1..=5);

    let resp: AicSearchResponse = client
        .get("https://api.artic.edu/api/v1/artworks/search")
        .header("AIC-User-Agent", "ArtDisplay/0.1 (Desktop Art Viewer)")
        .query(&[
            ("q", term),
            ("fields", "id,title,artist_display,date_display,medium_display,image_id"),
            ("limit", "20"),
            ("page", &page.to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("AIC search failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("AIC parse failed: {e}"))?;

    // Shuffle and try artworks until we get a valid image
    let mut artworks: Vec<&AicArtwork> = resp
        .data
        .iter()
        .filter(|a| a.image_id.is_some())
        .collect();

    if artworks.is_empty() {
        return Err("No AIC artworks with images".into());
    }

    // Shuffle to avoid always trying the same order
    use rand::seq::SliceRandom;
    artworks.shuffle(&mut rand::rng());

    for artwork in artworks.iter().take(5) {
        let image_id = match &artwork.image_id {
            Some(id) => id,
            None => continue,
        };

        // IIIF: request 843px wide (fast download, plenty for overlay)
        let image_url = format!(
            "{}/{}/full/843,/0/default.jpg",
            resp.config.iiif_url, image_id
        );

        let (image_bytes, mime) = match download_image(client, &image_url).await {
            Some(result) => result,
            None => continue, // try next artwork
        };

        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &image_bytes,
        );

        return Ok(Artwork {
            id: format!("aic-{}", artwork.id),
            title: strip_html(&artwork.title.clone().unwrap_or_else(|| "Untitled".into())),
            artist: artwork
                .artist_display
                .clone()
                .unwrap_or_else(|| "Unknown Artist".into()),
            date: artwork.date_display.clone().unwrap_or_default(),
            medium: artwork.medium_display.clone().unwrap_or_default(),
            source: "Art Institute of Chicago".into(),
            image_base64: format!("data:{mime};base64,{b64}"),
        });
    }

    Err("Could not find AIC artwork with valid image".into())
}

// ── Cleveland Museum of Art API ──

#[derive(Deserialize)]
struct CmaSearchResponse {
    data: Vec<CmaArtwork>,
}

#[derive(Deserialize)]
struct CmaArtwork {
    id: u64,
    title: Option<String>,
    #[serde(default)]
    creators: Vec<CmaCreator>,
    creation_date: Option<String>,
    technique: Option<String>,
    images: Option<CmaImages>,
}

#[derive(Deserialize)]
struct CmaCreator {
    description: Option<String>,
}

#[derive(Deserialize)]
struct CmaImages {
    web: Option<CmaImageVariant>,
}

#[derive(Deserialize)]
struct CmaImageVariant {
    url: Option<String>,
}

pub async fn fetch_cma_artwork(client: &Client) -> Result<Artwork, String> {
    let search_terms = [
        "painting", "landscape", "portrait", "impressionist", "modern",
        "still life", "abstract", "nature", "classical", "oil",
    ];
    let term = search_terms[rand::rng().random_range(0..search_terms.len())];
    let skip = rand::rng().random_range(0..100);

    let resp: CmaSearchResponse = client
        .get("https://openaccess-api.clevelandart.org/api/artworks/")
        .query(&[
            ("q", term),
            ("has_image", "1"),
            ("cc0", "1"),
            ("type", "Painting"),
            ("limit", "20"),
            ("skip", &skip.to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("CMA search failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("CMA parse failed: {e}"))?;

    use rand::seq::SliceRandom;
    let mut artworks: Vec<&CmaArtwork> = resp
        .data
        .iter()
        .filter(|a| {
            a.images
                .as_ref()
                .and_then(|i| i.web.as_ref())
                .and_then(|w| w.url.as_ref())
                .map(|u| !u.is_empty())
                .unwrap_or(false)
        })
        .collect();

    if artworks.is_empty() {
        return Err("No CMA artworks with images".into());
    }

    artworks.shuffle(&mut rand::rng());

    for artwork in artworks.iter().take(5) {
        let image_url = artwork
            .images
            .as_ref()
            .and_then(|i| i.web.as_ref())
            .and_then(|w| w.url.as_ref())
            .unwrap();

        let (image_bytes, mime) = match download_image(client, image_url).await {
            Some(result) => result,
            None => continue,
        };

        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &image_bytes,
        );

        let artist = artwork
            .creators
            .first()
            .and_then(|c| c.description.clone())
            .unwrap_or_else(|| "Unknown Artist".into());

        return Ok(Artwork {
            id: format!("cma-{}", artwork.id),
            title: strip_html(&artwork.title.clone().unwrap_or_else(|| "Untitled".into())),
            artist,
            date: artwork.creation_date.clone().unwrap_or_default(),
            medium: artwork.technique.clone().unwrap_or_default(),
            source: "Cleveland Museum of Art".into(),
            image_base64: format!("data:{mime};base64,{b64}"),
        });
    }

    Err("Could not find CMA artwork with valid image".into())
}

// ── National Gallery of Art (embedded catalog + IIIF) ──

#[derive(Deserialize)]
struct NgaCatalogEntry {
    uuid: String,
    title: String,
    artist: String,
    date: String,
    medium: String,
}

static NGA_CATALOG: std::sync::LazyLock<Vec<NgaCatalogEntry>> = std::sync::LazyLock::new(|| {
    let json = include_str!("../resources/nga_catalog.json");
    serde_json::from_str(json).expect("Failed to parse embedded NGA catalog")
});

pub async fn fetch_nga_artwork(client: &Client) -> Result<Artwork, String> {
    if NGA_CATALOG.is_empty() {
        return Err("NGA catalog is empty".into());
    }

    // Try up to 5 random entries
    for _ in 0..5 {
        let entry = &NGA_CATALOG[rand::rng().random_range(0..NGA_CATALOG.len())];

        let image_url = format!(
            "https://api.nga.gov/iiif/{}/full/!843,843/0/default.jpg",
            entry.uuid
        );

        let (image_bytes, mime) = match download_image(client, &image_url).await {
            Some(result) => result,
            None => continue,
        };

        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &image_bytes,
        );

        return Ok(Artwork {
            id: format!("nga-{}", entry.uuid),
            title: entry.title.clone(),
            artist: entry.artist.clone(),
            date: entry.date.clone(),
            medium: entry.medium.clone(),
            source: "National Gallery of Art".into(),
            image_base64: format!("data:{mime};base64,{b64}"),
        });
    }

    Err("Could not find NGA artwork with valid image".into())
}

/// Fetch a random artwork from any source
pub async fn fetch_random_artwork(client: &Client) -> Result<Artwork, String> {
    // Pick a random source (0=Met, 1=AIC, 2=CMA, 3=NGA)
    let source = rand::rng().random_range(0..4u32);

    let fetchers: [fn(&Client) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Artwork, String>> + Send + '_>>; 4] = [
        |c| Box::pin(fetch_met_artwork(c)),
        |c| Box::pin(fetch_aic_artwork(c)),
        |c| Box::pin(fetch_cma_artwork(c)),
        |c| Box::pin(fetch_nga_artwork(c)),
    ];

    let names = ["Met", "AIC", "CMA", "NGA"];

    // Try the selected source first, then fall back to others
    let order: Vec<usize> = {
        let start = source as usize;
        (0..4).map(|i| (start + i) % 4).collect()
    };

    let mut last_err = String::new();
    for &idx in &order {
        match fetchers[idx](client).await {
            Ok(art) => return Ok(art),
            Err(e) => {
                log::warn!("{} failed: {e}", names[idx]);
                last_err = e;
            }
        }
    }

    Err(format!("All sources failed. Last error: {last_err}"))
}
