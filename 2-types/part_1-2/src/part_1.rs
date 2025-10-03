use std::fmt;

#[derive(Debug, Clone)]
pub struct Title(String);
impl Title {
    pub fn new(s: impl Into<String>) -> Result<Self, &'static str> {
        let s = s.into().trim().to_string();
        if s.is_empty() { return Err("title cannot be empty"); }
        Ok(Self(s))
    }
}
impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
}

#[derive(Debug, Clone)]
pub struct Body(String);
impl Body {
    pub fn new(s: impl Into<String>) -> Result<Self, &'static str> {
        let s = s.into();
        if s.is_empty() { return Err("body cannot be empty"); }
        Ok(Self(s))
    }
}

#[derive(Debug, Clone)]
pub struct Author(String);
impl Author {
    pub fn new(s: impl Into<String>) -> Result<Self, &'static str> {
        let s = s.into().trim().to_string();
        if s.is_empty() { return Err("author cannot be empty"); }
        Ok(Self(s))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UtcMillis(pub i64);

#[derive(Debug, Clone)]
pub struct NewPost {
    title: Title,
    body: Body,
    author: Author,
    created_at: UtcMillis,
}
#[derive(Debug, Clone)]
pub struct UnmoderatedPost {
    title: Title,
    body: Body,
    author: Author,
    created_at: UtcMillis,
    published_at: UtcMillis,
}
#[derive(Debug, Clone)]
pub struct PublishedPost {
    title: Title,
    body: Body,
    author: Author,
    created_at: UtcMillis,
    published_at: UtcMillis,
    moderated_at: UtcMillis,
}

impl NewPost {
    pub fn new(title: Title, body: Body, author: Author, now: UtcMillis) -> Self {
        Self { title, body, author, created_at: now }
    }
    pub fn publish(self, at: UtcMillis) -> UnmoderatedPost {
        UnmoderatedPost {
            title: self.title,
            body: self.body,
            author: self.author,
            created_at: self.created_at,
            published_at: at,
        }
    }
}

impl UnmoderatedPost {
    pub fn allow(self, at: UtcMillis) -> PublishedPost {
        PublishedPost {
            title: self.title,
            body: self.body,
            author: self.author,
            created_at: self.created_at,
            published_at: self.published_at,
            moderated_at: at,
        }
    }
}

pub fn demo_transitions() {
    let title = Title::new("Hello Rust").unwrap();
    let body = Body::new("Type-state pattern example").unwrap();
    let author = Author::new("Alice").unwrap();
    let now = UtcMillis(1_700_000_000_000);

    let newp = NewPost::new(title, body, author, now);
    let unmod = newp.publish(UtcMillis(1_700_000_500_000));
    let _published = unmod.allow(UtcMillis(1_700_001_000_000));
    // let oops = newp.allow(UtcMillis(0));
}
