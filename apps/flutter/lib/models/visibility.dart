/// Mirrors tesseras-core Visibility enum.
enum Visibility {
  private('Private'),
  circle('Circle'),
  public('Public'),
  publicAfterDeath('Public After Death'),
  sealed_('Sealed');

  final String label;
  const Visibility(this.label);
}
