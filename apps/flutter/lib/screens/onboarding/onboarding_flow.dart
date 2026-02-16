import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'welcome_screen.dart';
import 'identity_screen.dart';
import 'ready_screen.dart';

/// PageView controller for the 3-step onboarding flow.
class OnboardingFlow extends StatefulWidget {
  const OnboardingFlow({super.key});

  @override
  State<OnboardingFlow> createState() => _OnboardingFlowState();
}

class _OnboardingFlowState extends State<OnboardingFlow> {
  final _pageController = PageController();
  int _currentPage = 0;

  @override
  void dispose() {
    _pageController.dispose();
    super.dispose();
  }

  void _goToPage(int page) {
    _pageController.animateToPage(
      page,
      duration: const Duration(milliseconds: 300),
      curve: Curves.easeInOut,
    );
  }

  void _next() {
    if (_currentPage < 2) {
      _goToPage(_currentPage + 1);
    }
  }

  void _back() {
    if (_currentPage > 0) {
      _goToPage(_currentPage - 1);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Shortcuts(
        shortcuts: {
          LogicalKeySet(LogicalKeyboardKey.escape): const _BackIntent(),
        },
        child: Actions(
          actions: {
            _BackIntent: CallbackAction<_BackIntent>(onInvoke: (_) {
              _back();
              return null;
            }),
          },
          child: Focus(
            autofocus: true,
            child: Column(
              children: [
                Expanded(
                  child: PageView(
                    controller: _pageController,
                    physics: const NeverScrollableScrollPhysics(),
                    onPageChanged: (page) =>
                        setState(() => _currentPage = page),
                    children: [
                      WelcomeScreen(onNext: _next),
                      IdentityScreen(onNext: _next, onBack: _back),
                      const ReadyScreen(),
                    ],
                  ),
                ),
                // Step indicator dots
                Padding(
                  padding: const EdgeInsets.only(bottom: 32),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: List.generate(3, (index) {
                      final isActive = index == _currentPage;
                      return AnimatedContainer(
                        duration: const Duration(milliseconds: 200),
                        margin: const EdgeInsets.symmetric(horizontal: 4),
                        width: isActive ? 24 : 8,
                        height: 8,
                        decoration: BoxDecoration(
                          color: isActive
                              ? Theme.of(context).colorScheme.primary
                              : Theme.of(context)
                                  .colorScheme
                                  .primary
                                  .withValues(alpha: 0.3),
                          borderRadius: BorderRadius.circular(4),
                        ),
                      );
                    }),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _BackIntent extends Intent {
  const _BackIntent();
}
