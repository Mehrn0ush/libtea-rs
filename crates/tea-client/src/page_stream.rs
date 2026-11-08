//! Opt-in pagination helpers.
//!
//! **Experimental:** may change in a 0.x minor release.
//!
//! The primary API remains caller-managed [`libtea_core::ListParams`]: replace
//! only `page_token` between calls. These helpers follow `next_page_token` with
//! hard limits so a hostile or buggy server cannot force unbounded fetches.

use std::future::Future;
use std::pin::Pin;

use futures_util::stream::{self, Stream};
use libtea_core::{ListParams, Page};

use crate::error::ClientError;

/// Limits for automatic page following.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PageFollowLimits {
    /// Maximum number of HTTP list pages to fetch (including the first).
    pub max_pages: u32,
    /// Stop after the page on which the running item count reaches this value.
    ///
    /// That page is yielded whole, so the total can exceed this by up to one
    /// page (each page is still bounded by [`crate::Limits::max_json_bytes`]).
    pub max_items: u64,
}

impl Default for PageFollowLimits {
    fn default() -> Self {
        Self {
            max_pages: 32,
            max_items: 10_000,
        }
    }
}

/// Why page following stopped.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageFollowStop {
    /// Server reported no further page (`has_next` false or empty token).
    Exhausted,
    /// Hit [`PageFollowLimits::max_pages`].
    MaxPages,
    /// Hit [`PageFollowLimits::max_items`].
    MaxItems,
    /// Server repeated the same `next_page_token` (loop guard).
    RepeatedToken,
}

/// One page in a followed list, plus whether following should continue.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug)]
#[non_exhaustive]
pub struct FollowedPage<T> {
    /// Decoded page body.
    pub page: Page<T>,
    /// Why this was the last page, if following stopped.
    pub stop: Option<PageFollowStop>,
}

impl PageFollowLimits {
    /// Limits with an explicit page and item bound.
    #[must_use]
    pub fn new(max_pages: u32, max_items: u64) -> Self {
        Self {
            max_pages,
            max_items,
        }
    }

    /// Set [`Self::max_pages`].
    #[must_use]
    pub fn with_max_pages(mut self, max_pages: u32) -> Self {
        self.max_pages = max_pages;
        self
    }

    /// Set [`Self::max_items`].
    #[must_use]
    pub fn with_max_items(mut self, max_items: u64) -> Self {
        self.max_items = max_items;
        self
    }
}

type PageFuture<T> = Pin<Box<dyn Future<Output = Result<Page<T>, ClientError>> + Send>>;

/// Follow `page_token` until exhausted or a limit / loop is hit.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// `fetch` receives the current [`ListParams`] (caller should keep filters and
/// replace only the token the helper sets). `S` is the list endpoint's sort
/// type (for example `ProductSort`).
pub fn follow_pages<T, S, F>(
    params: ListParams<S>,
    limits: PageFollowLimits,
    fetch: F,
) -> impl Stream<Item = Result<FollowedPage<T>, ClientError>>
where
    T: Send + 'static,
    S: Clone + Send + 'static,
    F: FnMut(ListParams<S>) -> PageFuture<T> + Send + 'static,
{
    stream::unfold(
        FollowState {
            params,
            limits,
            pages: 0,
            items: 0,
            last_token: None,
            done: false,
            fetch,
        },
        |mut state| async move {
            if state.done {
                return None;
            }
            if state.pages >= state.limits.max_pages {
                return None;
            }
            let page = match (state.fetch)(state.params.clone()).await {
                Ok(page) => page,
                Err(error) => {
                    state.done = true;
                    return Some((Err(error), state));
                }
            };
            state.pages += 1;
            state.items = state.items.saturating_add(page.results.len() as u64);

            let mut stop = None;
            if state.items >= state.limits.max_items {
                stop = Some(PageFollowStop::MaxItems);
                state.done = true;
            } else if state.pages >= state.limits.max_pages {
                stop = Some(PageFollowStop::MaxPages);
                state.done = true;
            } else if !page.has_next {
                stop = Some(PageFollowStop::Exhausted);
                state.done = true;
            } else {
                match page.next_page_token.clone() {
                    // An empty token cannot select a next page; following it
                    // would re-request the first page.
                    Some(token) if token.is_empty() => {
                        stop = Some(PageFollowStop::Exhausted);
                        state.done = true;
                    }
                    Some(token) if state.last_token.as_ref() == Some(&token) => {
                        stop = Some(PageFollowStop::RepeatedToken);
                        state.done = true;
                    }
                    Some(token) => {
                        state.last_token = Some(token.clone());
                        state.params.page_token = Some(token);
                    }
                    None => {
                        stop = Some(PageFollowStop::Exhausted);
                        state.done = true;
                    }
                }
            }

            Some((
                Ok(FollowedPage {
                    page,
                    stop: if state.done {
                        stop.or(Some(PageFollowStop::Exhausted))
                    } else {
                        None
                    },
                }),
                state,
            ))
        },
    )
}

struct FollowState<S, F> {
    params: ListParams<S>,
    limits: PageFollowLimits,
    pages: u32,
    items: u64,
    last_token: Option<String>,
    done: bool,
    fetch: F,
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    #[tokio::test]
    async fn stops_on_repeated_token() {
        let stream = follow_pages(
            ListParams::<()>::default(),
            PageFollowLimits::default(),
            |_: ListParams<()>| {
                Box::pin(async { Ok(Page::new(vec![1_u32], true, Some("same".into()))) })
            },
        );
        futures_util::pin_mut!(stream);
        let first = stream.next().await.unwrap().unwrap();
        assert!(first.stop.is_none());
        let second = stream.next().await.unwrap().unwrap();
        assert_eq!(second.stop, Some(PageFollowStop::RepeatedToken));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn respects_max_pages() {
        let limits = PageFollowLimits {
            max_pages: 2,
            max_items: 10_000,
        };
        let stream = follow_pages(ListParams::<()>::default(), limits, |_: ListParams<()>| {
            Box::pin(async {
                Ok(Page::new(
                    vec![1_u32],
                    true,
                    Some(format!("{}", uuid_like())),
                ))
            })
        });
        futures_util::pin_mut!(stream);
        assert!(stream.next().await.unwrap().unwrap().stop.is_none());
        let last = stream.next().await.unwrap().unwrap();
        assert_eq!(last.stop, Some(PageFollowStop::MaxPages));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn empty_next_token_is_exhausted_not_followed() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter = std::sync::Arc::clone(&calls);
        let stream = follow_pages(
            ListParams::<()>::default(),
            PageFollowLimits::default(),
            move |_: ListParams<()>| {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Box::pin(async { Ok(Page::new(vec![1_u32], true, Some(String::new()))) })
            },
        );
        futures_util::pin_mut!(stream);
        let only = stream.next().await.unwrap().unwrap();
        assert_eq!(only.stop, Some(PageFollowStop::Exhausted));
        assert!(stream.next().await.is_none());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    fn uuid_like() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(1);
        N.fetch_add(1, Ordering::Relaxed)
    }
}
