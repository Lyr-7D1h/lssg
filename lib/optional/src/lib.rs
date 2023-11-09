pub trait Optional<T> {
    fn from_optional(&mut self, t: T);
}
