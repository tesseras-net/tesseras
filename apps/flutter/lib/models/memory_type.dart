/// Mirrors tesseras-core MemoryType enum.
enum MemoryType {
  moment('Moment'),
  reflection('Reflection'),
  daily('Daily'),
  relation('Relation'),
  object('Object');

  final String label;
  const MemoryType(this.label);
}
