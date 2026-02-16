import 'package:flutter/material.dart';

/// Navigation sidebar widget — 220px wide, permanent.
class Sidebar extends StatelessWidget {
  final int selectedIndex;
  final ValueChanged<int> onItemSelected;
  final VoidCallback onCreateMemory;

  const Sidebar({
    super.key,
    required this.selectedIndex,
    required this.onItemSelected,
    required this.onCreateMemory,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Container(
      width: 220,
      color: colorScheme.surfaceContainerLow,
      child: Column(
        children: [
          const SizedBox(height: 16),
          // App title
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              children: [
                Icon(Icons.memory, color: colorScheme.primary, size: 28),
                const SizedBox(width: 8),
                Text(
                  'Tesseras',
                  style: Theme.of(context).textTheme.titleLarge?.copyWith(
                        color: colorScheme.primary,
                        fontWeight: FontWeight.bold,
                      ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),
          // Navigation items
          _SidebarItem(
            icon: Icons.photo_library,
            label: 'Timeline',
            selected: selectedIndex == 0,
            onTap: () => onItemSelected(0),
            tooltip: 'Timeline (Ctrl+1)',
          ),
          _SidebarItem(
            icon: Icons.hub,
            label: 'Network',
            selected: selectedIndex == 1,
            onTap: () => onItemSelected(1),
            tooltip: 'Network (Ctrl+2)',
          ),
          _SidebarItem(
            icon: Icons.settings,
            label: 'Settings',
            selected: selectedIndex == 2,
            onTap: () => onItemSelected(2),
            tooltip: 'Settings (Ctrl+3)',
          ),
          const Spacer(),
          // Create memory button
          Padding(
            padding: const EdgeInsets.all(16),
            child: SizedBox(
              width: double.infinity,
              child: Tooltip(
                message: 'New Memory (Ctrl+N)',
                child: FilledButton.icon(
                  onPressed: onCreateMemory,
                  icon: const Icon(Icons.add),
                  label: const Text('New Memory'),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _SidebarItem extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;
  final String tooltip;

  const _SidebarItem({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
    required this.tooltip,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Tooltip(
      message: tooltip,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
        child: Material(
          color: selected
              ? colorScheme.primaryContainer.withValues(alpha: 0.4)
              : Colors.transparent,
          borderRadius: BorderRadius.circular(12),
          child: InkWell(
            borderRadius: BorderRadius.circular(12),
            onTap: onTap,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
              child: Row(
                children: [
                  Icon(
                    icon,
                    size: 22,
                    color: selected
                        ? colorScheme.primary
                        : colorScheme.onSurfaceVariant,
                  ),
                  const SizedBox(width: 12),
                  Text(
                    label,
                    style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                          color: selected
                              ? colorScheme.primary
                              : colorScheme.onSurfaceVariant,
                          fontWeight:
                              selected ? FontWeight.w600 : FontWeight.normal,
                        ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
