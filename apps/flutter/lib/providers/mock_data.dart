import 'package:flutter/material.dart';
import '../models/memory.dart';
import '../models/memory_type.dart';
import '../models/visibility.dart' as v;
import '../models/network_event.dart';

/// Mock identity for onboarding and settings.
class MockIdentity {
  final String name;
  final String nodeIdHex;
  final String ed25519PublicKeyHex;
  final String mldsaPublicKeyHex;
  final String createdAt;
  final Color avatarColor;
  final String? avatarImagePath;

  const MockIdentity({
    required this.name,
    required this.nodeIdHex,
    required this.ed25519PublicKeyHex,
    required this.mldsaPublicKeyHex,
    required this.createdAt,
    required this.avatarColor,
    this.avatarImagePath,
  });
}

const mockIdentity = MockIdentity(
  name: 'Ivan',
  nodeIdHex: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
  ed25519PublicKeyHex:
      '7f3a8b2c1d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f01',
  mldsaPublicKeyHex:
      '9c4d1e8f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d',
  createdAt: '2026-02-14T10:30:00Z',
  avatarColor: Colors.indigo,
);

/// 12 mock memories covering all MemoryType and Visibility values.
final mockMemories = <Memory>[
  // #1: Moment, Public, JPEG, location+people+tags
  const Memory(
    hash: 'b1a2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f001',
    tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
    type: MemoryType.moment,
    visibility: v.Visibility.public,
    context:
        'A beautiful sunset from our vacation in Paraty. The kids were playing on the beach.',
    createdAt: '2026-02-14T18:30:00Z',
    tags: ['family', 'vacation', '2026'],
    location: 'Paraty, RJ, Brasil',
    people: ['Maria - spouse', 'Lucas - son'],
    language: 'pt',
    mediaType: 'jpeg',
  ),
  // #2: Moment, Private, JPEG, location, no people
  const Memory(
    hash: 'c2b3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f002',
    tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
    type: MemoryType.moment,
    visibility: v.Visibility.private,
    context: 'Morning coffee at the cabin. Perfect silence.',
    createdAt: '2026-02-13T07:45:00Z',
    tags: ['nature', 'morning'],
    location: 'Serra da Mantiqueira, SP',
    language: 'en',
    mediaType: 'jpeg',
  ),
  // #3: Reflection, Circle, TXT, no location, no people
  const Memory(
    hash: 'd3c4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f003',
    tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
    type: MemoryType.reflection,
    visibility: v.Visibility.circle,
    context:
        'On the nature of time: We preserve memories not because we fear forgetting, but because we believe in the future. Each tessera is an act of hope.',
    createdAt: '2026-02-12T22:10:00Z',
    tags: ['philosophy', 'time'],
    language: 'en',
    mediaType: 'txt',
  ),
  // #4: Daily, Public, JPEG, no location, no people, no tags
  const Memory(
    hash: 'e4d5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f004',
    tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
    type: MemoryType.daily,
    visibility: v.Visibility.public,
    context: 'Worked on the garden today. Planted tomatoes and basil.',
    createdAt: '2026-02-11T16:20:00Z',
    language: 'en',
    mediaType: 'jpeg',
  ),
  // #5: Relation, Private, JPEG, location, 1 person
  const Memory(
    hash: 'f5e6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f005',
    tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
    type: MemoryType.relation,
    visibility: v.Visibility.private,
    context: 'My grandfather teaching me to fish at the lake.',
    createdAt: '2026-02-10T10:00:00Z',
    tags: ['family', 'grandfather', 'childhood'],
    location: 'Lago Azul, MG',
    people: ['Joaquim - grandfather'],
    language: 'pt',
    mediaType: 'jpeg',
  ),
  // #6: Object, Public, PNG, no location, no people
  const Memory(
    hash: 'a6f7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f006',
    tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
    type: MemoryType.object,
    visibility: v.Visibility.public,
    context:
        'The family pocket watch, passed down from great-grandmother. Still ticking after 120 years.',
    createdAt: '2026-02-09T14:30:00Z',
    tags: ['heirloom', 'watch'],
    language: 'en',
    mediaType: 'png',
  ),
  // #7: Moment, Sealed (2030-01-01), JPEG, location, 3 people
  Memory(
    hash: 'b7a8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f007',
    tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
    type: MemoryType.moment,
    visibility: v.Visibility.sealed_,
    context: 'A secret surprise party. Do not open until 2030!',
    createdAt: '2026-02-08T20:00:00Z',
    tags: ['surprise', 'party', 'sealed'],
    location: 'Home, Sao Paulo',
    people: ['Maria - spouse', 'Lucas - son', 'Ana - daughter'],
    language: 'pt',
    mediaType: 'jpeg',
    sealedOpenAfter: DateTime(2030, 1, 1),
  ),
  // #8: Moment, PublicAfterDeath (5y), JPEG, no location, no people, no tags
  const Memory(
    hash: 'c8b9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f008',
    tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
    type: MemoryType.moment,
    visibility: v.Visibility.publicAfterDeath,
    context: 'A letter to my future grandchildren.',
    createdAt: '2026-02-07T11:15:00Z',
    language: 'en',
    mediaType: 'jpeg',
    publicAfterDeathYears: 5,
  ),
  // #9: Reflection, Private, TXT, no location, no people
  const Memory(
    hash: 'd9c0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f009',
    tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
    type: MemoryType.reflection,
    visibility: v.Visibility.private,
    context:
        'What I have learned about resilience: systems that survive are not the strongest, but the most adaptable. Tesseras embodies this principle.',
    createdAt: '2026-02-06T09:30:00Z',
    tags: ['resilience', 'systems'],
    language: 'en',
    mediaType: 'txt',
  ),
  // #10: Moment, Public, JPEG, location, no people
  const Memory(
    hash: 'e0d1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f010',
    tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
    type: MemoryType.moment,
    visibility: v.Visibility.public,
    context: 'First snow of the year in the mountains.',
    createdAt: '2026-02-05T08:00:00Z',
    tags: ['nature', 'snow', 'winter'],
    location: 'Campos do Jordao, SP',
    language: 'pt',
    mediaType: 'jpeg',
  ),
  // #11: Daily, Circle, JPEG, no location, 1 person, no tags
  const Memory(
    hash: 'f1e2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f011',
    tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
    type: MemoryType.daily,
    visibility: v.Visibility.circle,
    context: 'Cooking dinner with Lucas. He made pasta from scratch!',
    createdAt: '2026-02-04T19:45:00Z',
    people: ['Lucas - son'],
    language: 'en',
    mediaType: 'jpeg',
  ),
  // #12: Moment, Public, JPEG, location, 2 people
  const Memory(
    hash: 'a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f012',
    tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
    type: MemoryType.moment,
    visibility: v.Visibility.public,
    context: 'New Year celebration at the beach. Fireworks over the ocean.',
    createdAt: '2026-01-01T00:05:00Z',
    tags: ['celebration', 'new-year', '2026'],
    location: 'Copacabana, RJ, Brasil',
    people: ['Maria - spouse', 'Ana - daughter'],
    language: 'pt',
    mediaType: 'jpeg',
  ),
];

/// 5 connected peers for network screen.
const mockConnectedPeers = <ConnectedPeer>[
  ConnectedPeer(
    nodeId: 'a3f8b2c1e7d4',
    address: '192.168.1.5:4433',
    lastSeen: '2m ago',
  ),
  ConnectedPeer(
    nodeId: '7e2df491c8a3',
    address: '45.32.100.12:4433',
    lastSeen: '5m ago',
  ),
  ConnectedPeer(
    nodeId: 'c4a19e7b2d6f',
    address: '10.0.0.8:4433',
    lastSeen: '12m ago',
  ),
  ConnectedPeer(
    nodeId: 'b8d3e7f1a2c5',
    address: '203.0.113.42:4433',
    lastSeen: '18m ago',
  ),
  ConnectedPeer(
    nodeId: 'f6a9c2e8d4b7',
    address: '198.51.100.7:4433',
    lastSeen: '25m ago',
  ),
];

/// 12 network events for the event log.
const mockNetworkEvents = <NetworkEvent>[
  NetworkEvent(
    timestamp: '14:32',
    type: NetworkEventType.peerConnected,
    details: 'a3f8..b2c1',
  ),
  NetworkEvent(
    timestamp: '14:31',
    type: NetworkEventType.attestationReceived,
    details: 'frag#847',
  ),
  NetworkEvent(
    timestamp: '14:28',
    type: NetworkEventType.replicationComplete,
    details: 'mem#12',
  ),
  NetworkEvent(
    timestamp: '14:25',
    type: NetworkEventType.peerDisconnected,
    details: '7e2d..f491',
  ),
  NetworkEvent(
    timestamp: '14:20',
    type: NetworkEventType.repairTriggered,
    details: 'frag#831',
  ),
  NetworkEvent(
    timestamp: '14:15',
    type: NetworkEventType.bootstrapComplete,
    details: 'boot1.tesseras.net',
  ),
  NetworkEvent(
    timestamp: '14:10',
    type: NetworkEventType.peerConnected,
    details: 'c4a1..9e7b',
  ),
  NetworkEvent(
    timestamp: '14:05',
    type: NetworkEventType.attestationReceived,
    details: 'frag#845',
  ),
  NetworkEvent(
    timestamp: '13:58',
    type: NetworkEventType.replicationComplete,
    details: 'mem#11',
  ),
  NetworkEvent(
    timestamp: '13:52',
    type: NetworkEventType.peerConnected,
    details: 'b8d3..e7f1',
  ),
  NetworkEvent(
    timestamp: '13:45',
    type: NetworkEventType.repairTriggered,
    details: 'frag#828',
  ),
  NetworkEvent(
    timestamp: '13:40',
    type: NetworkEventType.peerDisconnected,
    details: 'f6a9..c2e8',
  ),
];

/// Mock network stats for display.
class MockNetworkStats {
  final int connectedPeers;
  final int dhtEntries;
  final bool bootstrapped;
  final String uptime;
  final String natStatus;
  final int totalFragments;
  final int healthyFragments;
  final int repairingFragments;
  final int replicationFactor;
  final int storageUsedMB;

  const MockNetworkStats({
    required this.connectedPeers,
    required this.dhtEntries,
    required this.bootstrapped,
    required this.uptime,
    required this.natStatus,
    required this.totalFragments,
    required this.healthyFragments,
    required this.repairingFragments,
    required this.replicationFactor,
    required this.storageUsedMB,
  });
}

const mockNetworkStats = MockNetworkStats(
  connectedPeers: 12,
  dhtEntries: 1204,
  bootstrapped: true,
  uptime: '2h 14m',
  natStatus: 'open',
  totalFragments: 847,
  healthyFragments: 831,
  repairingFragments: 16,
  replicationFactor: 5,
  storageUsedMB: 142,
);
