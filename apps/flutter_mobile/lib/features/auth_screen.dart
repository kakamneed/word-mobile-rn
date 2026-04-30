import 'package:flutter/material.dart';

import '../supabase/auth_session_manager.dart';

class AuthScreen extends StatefulWidget {
  const AuthScreen({super.key, this.onAuthChanged});

  final void Function(AuthAccountState authState)? onAuthChanged;

  @override
  State<AuthScreen> createState() => _AuthScreenState();
}

class _AuthScreenState extends State<AuthScreen> {
  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  final _sessionManager = AuthSessionManager();

  bool _loading = true;
  bool _submitting = false;
  bool _signupMode = false;
  AuthAccountState _authState = const AuthAccountState.uninitialized();

  @override
  void initState() {
    super.initState();
    _restore();
  }

  @override
  void dispose() {
    _emailController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  Future<void> _restore() async {
    setState(() {
      _loading = true;
      _authState = const AuthAccountState.checking();
    });
    final authState = await _sessionManager.resolveStartupState();
    if (mounted) {
      setState(() {
        _authState = authState;
        _loading = false;
      });
    }
  }

  Future<void> _submit() async {
    setState(() {
      _submitting = true;
      _authState = const AuthAccountState.checking();
    });

    final email = _emailController.text.trim();
    final password = _passwordController.text;
    final authState = _signupMode
        ? await _sessionManager.signUp(email: email, password: password)
        : await _sessionManager.signIn(email: email, password: password);
    if (mounted) {
      setState(() {
        _authState = authState;
      });
    }
    widget.onAuthChanged?.call(authState);
    if (mounted) {
      setState(() {
        _submitting = false;
      });
    }
  }

  Future<void> _signOut() async {
    setState(() {
      _submitting = true;
      _authState = const AuthAccountState.checking();
    });
    final authState = await _sessionManager.signOutRetainingLocalData();
    if (mounted) {
      setState(() {
        _authState = authState;
      });
    }
    widget.onAuthChanged?.call(authState);
    if (mounted) {
      setState(() {
        _submitting = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_authState.phase == AuthAccountPhase.notConfigured) {
      return Scaffold(
        appBar: AppBar(title: const Text('Account')),
        body: const Padding(
          padding: EdgeInsets.all(24),
          child: Text(
            'Supabase is not configured yet. Provide SUPABASE_URL and SUPABASE_ANON_KEY via dart-define before testing auth flows.',
          ),
        ),
      );
    }

    return Scaffold(
      appBar: AppBar(title: const Text('Account')),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: const EdgeInsets.all(16),
              children: [
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          _accountTitle,
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                        const SizedBox(height: 8),
                        Text(
                          _authState.userEmail ??
                              _authState.message ??
                              'No active session',
                        ),
                        const SizedBox(height: 8),
                        Text(
                          'Local study: '
                          '${_authState.allowsLocalStudy ? 'available' : 'blocked'}',
                        ),
                        Text(
                          'Cloud work: '
                          '${_authState.allowsCloudWork ? 'available' : 'disabled'}',
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: 16),
                TextField(
                  controller: _emailController,
                  keyboardType: TextInputType.emailAddress,
                  decoration: const InputDecoration(
                    labelText: 'Email',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: _passwordController,
                  obscureText: true,
                  decoration: const InputDecoration(
                    labelText: 'Password',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                FilledButton(
                  onPressed: _submitting ? null : _submit,
                  child: Text(
                    _submitting
                        ? 'Submitting...'
                        : (_signupMode ? 'Create account' : 'Sign in'),
                  ),
                ),
                const SizedBox(height: 8),
                TextButton(
                  onPressed: _submitting
                      ? null
                      : () {
                          setState(() {
                            _signupMode = !_signupMode;
                          });
                        },
                  child: Text(
                    _signupMode
                        ? 'Switch to sign in'
                        : 'Switch to signup',
                  ),
                ),
                const SizedBox(height: 8),
                OutlinedButton(
                  onPressed: _submitting ? null : _signOut,
                  child: const Text('Sign out'),
                ),
                if (_authState.message != null) ...[
                  const SizedBox(height: 16),
                  Text(_authState.message!),
                ],
              ],
            ),
    );
  }

  String get _accountTitle {
    return switch (_authState.phase) {
      AuthAccountPhase.uninitialized => 'Account not checked',
      AuthAccountPhase.checking => 'Checking account',
      AuthAccountPhase.notConfigured => 'Supabase not configured',
      AuthAccountPhase.guestLocalOnly => 'Guest local-only mode',
      AuthAccountPhase.signedInActive => 'Signed in',
      AuthAccountPhase.signedInExpired => 'Session expired',
      AuthAccountPhase.signedOutRetainedLocal =>
        'Signed out, local data retained',
      AuthAccountPhase.accountDeletedOrRevoked => 'Account access revoked',
      AuthAccountPhase.error => 'Account error',
    };
  }
}
