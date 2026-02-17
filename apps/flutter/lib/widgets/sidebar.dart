import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../l10n/app_localizations.dart';
import '../providers/network_provider.dart';

/// Navigation sidebar widget — 220px wide, permanent.
class Sidebar extends ConsumerWidget {
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
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final l = AppLocalizations.of(context);
    final networkAsync = ref.watch(networkProvider);
    final peerCount = networkAsync.value?.stats.peerCount ?? 0;

    return Container(
      width: 220,
      decoration: BoxDecoration(
        color: theme.colorScheme.card,
        border: Border(
          right: BorderSide(color: theme.colorScheme.border),
        ),
      ),
      child: Column(
        children: [
          const SizedBox(height: 16),
          // App title with connection status
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              children: [
                Stack(
                  children: [
                    ClipRRect(
                      borderRadius: BorderRadius.circular(6),
                      child: Image.asset('assets/logo.png', width: 32, height: 32),
                    ),
                    Positioned(
                      right: -2,
                      bottom: -2,
                      child: Container(
                        width: 12,
                        height: 12,
                        decoration: BoxDecoration(
                          color: peerCount > 0
                              ? const Color(0xFF4CAF50)
                              : const Color(0xFFF44336),
                          shape: BoxShape.circle,
                          border: Border.all(
                            color: theme.colorScheme.card,
                            width: 2,
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(l.appTitle).semiBold,
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),
          // Navigation items
          _SidebarItem(
            icon: Icons.photo_library,
            label: l.sidebarTimeline,
            selected: selectedIndex == 0,
            onTap: () => onItemSelected(0),
          ),
          _SidebarItem(
            icon: Icons.hub,
            label: l.sidebarNetwork,
            selected: selectedIndex == 1,
            onTap: () => onItemSelected(1),
          ),
          _SidebarItem(
            icon: Icons.settings,
            label: l.sidebarSettings,
            selected: selectedIndex == 2,
            onTap: () => onItemSelected(2),
          ),
          const Spacer(),
          // Create memory button
          Padding(
            padding: const EdgeInsets.all(16),
            child: SizedBox(
              width: double.infinity,
              child: Button.primary(
                onPressed: onCreateMemory,
                leading: const Icon(Icons.add, size: 18),
                child: Text(l.sidebarNewMemory),
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

  const _SidebarItem({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      child: Button(
        style: ButtonStyle(
          variance: selected ? ButtonVariance.secondary : ButtonVariance.ghost,
        ),
        onPressed: onTap,
        alignment: Alignment.centerLeft,
        child: Row(
          children: [
            Icon(
              icon,
              size: 20,
              color: selected
                  ? theme.colorScheme.primary
                  : theme.colorScheme.mutedForeground,
            ),
            const SizedBox(width: 12),
            Text(
              label,
              style: TextStyle(
                color: selected
                    ? theme.colorScheme.primary
                    : theme.colorScheme.mutedForeground,
                fontWeight: selected ? FontWeight.w600 : FontWeight.normal,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
