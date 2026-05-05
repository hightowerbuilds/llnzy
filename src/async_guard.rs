#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AsyncRequestToken(u64);

impl AsyncRequestToken {
    pub fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Default)]
pub struct AsyncRequestCounter {
    next: u64,
}

impl AsyncRequestCounter {
    pub fn next_token(&mut self) -> AsyncRequestToken {
        self.next = self.next.wrapping_add(1).max(1);
        AsyncRequestToken(self.next)
    }
}

pub fn is_current_request<T: PartialEq>(current: Option<&T>, candidate: &T) -> bool {
    current == Some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_tokens_are_unique_and_monotonic_until_wrap() {
        let mut counter = AsyncRequestCounter::default();

        let first = counter.next_token();
        let second = counter.next_token();

        assert_ne!(first, second);
        assert_eq!(first.raw(), 1);
        assert_eq!(second.raw(), 2);
    }

    #[test]
    fn current_request_check_rejects_missing_or_stale_candidates() {
        let current = AsyncRequestToken(2);

        assert!(is_current_request(Some(&current), &current));
        assert!(!is_current_request(Some(&current), &AsyncRequestToken(1)));
        assert!(!is_current_request::<AsyncRequestToken>(None, &current));
    }
}
