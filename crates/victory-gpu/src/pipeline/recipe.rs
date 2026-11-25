//! Pipeline recipes: what a title pipeline is made from (its shader
//! containers and its graphics state), written under the pipeline cache
//! folder at the pipeline's first use, so that `victory warm` can make the
//! translations and host pipelines of a new build without the title.
//!
//! A build that changes the translator invalidates both caches: every
//! translation is keyed by the build, every library entry by the translated
//! code. A run after such a change spent about 30 minutes at 1-2 fps while
//! ~1,900 pipelines were made again in the world; made beforehand from the
//! recipes, the first run of a new build finds them in the library.
//!
//! Layout: `recipes/<key>` (the 32-hex key the early list uses) and
//! `containers/<hash>` (a shader container by its logged 16-hex hash).
//! `VICTORY_RECIPES=0` writes none.

use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};

use super::Shader;
use super::host::{GraphicsState, Reader, Writer};
use super::make::Pending;

const RECIPES: &str = "recipes";
const CONTAINERS: &str = "containers";
const MAGIC: &[u8; 4] = b"VRC1";

struct Job {
    key: String,
    shaders: Vec<Arc<Shader>>,
    graphics: Option<Vec<u8>>,
}

static SEND: OnceLock<Option<Mutex<mpsc::Sender<Job>>>> = OnceLock::new();

/// A draw or dispatch first requested the pipeline `key`: its recipe is
/// written (on a thread of its own, once per key over all runs).
pub(crate) fn note(key: &str, shaders: &[Arc<Shader>], graphics: Option<&GraphicsState>) {
    if shaders.is_empty() {
        return;
    }
    let send = SEND.get_or_init(|| {
        if std::env::var_os("VICTORY_RECIPES").is_some_and(|v| v == "0") {
            return None;
        }
        let dir = super::library::dir()?.clone();
        let (tx, rx) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("pipeline recipes".into())
            .spawn(move || {
                let mut warned = false;
                for job in rx {
                    if let Err(e) = write(&dir, &job)
                        && !std::mem::replace(&mut warned, true)
                    {
                        tracing::warn!(error = %e, "pipeline recipe not written");
                    }
                }
            })
            .ok()?;
        Some(Mutex::new(tx))
    });
    let Some(send) = send else { return };
    let job = Job {
        key: key.to_owned(),
        shaders: shaders.to_vec(),
        graphics: graphics.map(GraphicsState::encode),
    };
    let _ = send.lock().unwrap_or_else(|e| e.into_inner()).send(job);
}

fn encode(job: &Job) -> Vec<u8> {
    let mut w = Writer::default();
    w.0.extend_from_slice(MAGIC);
    w.u(job.shaders.len() as u32);
    for s in &job.shaders {
        w.bytes(s.stage.as_bytes());
        w.bytes(s.hash.as_bytes());
    }
    match &job.graphics {
        Some(g) => {
            w.u(1);
            w.bytes(g);
        }
        None => w.u(0),
    }
    w.0
}

fn write(dir: &Path, job: &Job) -> io::Result<()> {
    let recipes = dir.join(RECIPES);
    let path = recipes.join(&job.key);
    if path.exists() {
        return Ok(());
    }
    let containers = dir.join(CONTAINERS);
    std::fs::create_dir_all(&recipes)?;
    std::fs::create_dir_all(&containers)?;
    for s in &job.shaders {
        let c = containers.join(&s.hash);
        if !c.exists() {
            write_new(&c, &s.container)?;
        }
    }
    // The recipe last: one that is there has its containers.
    write_new(&path, &encode(job))
}

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// The stage name as the pipeline table spells it.
fn stage(name: &[u8]) -> Option<&'static str> {
    Some(match name {
        b"vertex" => "vertex",
        b"pixel" => "pixel",
        b"domain" => "domain",
        b"hull" => "hull",
        b"geometry" => "geometry",
        b"amplification" => "amplification",
        b"mesh" => "mesh",
        b"compute" => "compute",
        _ => return None,
    })
}

/// The pipeline a recipe describes; `None` for one that does not read.
fn read(dir: &Path, bytes: &[u8]) -> Option<Pending> {
    let rest = bytes.strip_prefix(MAGIC)?;
    let mut r = Reader(rest);
    let n = r.u()? as usize;
    if n == 0 || n > 8 {
        return None;
    }
    let mut shaders = Vec::with_capacity(n);
    for _ in 0..n {
        let stage = stage(r.bytes()?)?;
        let hash = std::str::from_utf8(r.bytes()?).ok()?;
        if hash.is_empty() || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let container = std::fs::read(dir.join(CONTAINERS).join(hash)).ok()?;
        shaders.push(super::shader(stage, &container));
    }
    let graphics = match r.u()? {
        0 => None,
        1 => Some(Box::new(GraphicsState::decode(r.bytes()?)?)),
        _ => return None,
    };
    r.0.is_empty().then(|| Pending::new(shaders, graphics))
}

/// What [`warm`] did.
#[derive(Debug, Default)]
pub struct Warmed {
    /// Recipes found.
    pub recipes: usize,
    /// Recipes that did not read (or whose containers are missing).
    pub unreadable: usize,
    /// Recipes read back under another key than their name: the state
    /// they describe changed since (their host pipeline may not be the
    /// one the title asks for).
    pub other_key: usize,
    /// Pipelines with a host pipeline (made now or found in the library).
    pub made: usize,
    /// Pipelines refused or with a shader that does not translate.
    pub refused: usize,
}

/// Makes the host pipeline of every recipe under the pipeline cache with
/// `jobs` threads, then saves the library; `progress(done, total)` is
/// called as pipelines are made.
pub(crate) fn warm(jobs: usize, progress: &(dyn Fn(usize, usize) + Sync)) -> Warmed {
    let mut warmed = Warmed::default();
    let Some(dir) = super::library::dir() else {
        return warmed;
    };
    let mut pending = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir.join(RECIPES)) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.len() != 32 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
                continue;
            }
            warmed.recipes += 1;
            match std::fs::read(entry.path()).ok().and_then(|b| read(dir, &b)) {
                Some(p) => {
                    if p.key() != name {
                        warmed.other_key += 1;
                    }
                    pending.push(Arc::new(p));
                }
                None => warmed.unreadable += 1,
            }
        }
    }
    let total = pending.len();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let made = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..jobs.clamp(1, 64) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(p) = pending.get(i) else { break };
                    if super::make::now(p) {
                        made.fetch_add(1, Ordering::Relaxed);
                    }
                    let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                    progress(d, total);
                    // Save as we go: an interrupted warm keeps its work.
                    super::library::save(false);
                }
            });
        }
    });
    warmed.made = made.into_inner();
    warmed.refused = total - warmed.made;
    super::library::save(true);
    warmed
}
