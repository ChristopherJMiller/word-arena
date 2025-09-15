#!/bin/bash

# Test script for authentication and matchmaking

WS_URL="ws://localhost:8080/ws"
echo "Testing Word Arena Authentication and Matchmaking"
echo "================================================"

echo ""
echo "1. Testing unauthenticated queue join (should fail)"
echo "Connecting to WebSocket..."

# Test 1: Try to join queue without authentication
cat << 'EOF' | websocat $WS_URL
"JoinQueue"
EOF

echo ""
echo "2. Testing with fake authentication token"
echo "Connecting to WebSocket..."

# Test 2: Authenticate with fake token and join queue
(
  echo '{"Authenticate":{"token":"user1:alice@example.com:Alice"}}'
  sleep 1
  echo '"JoinQueue"'
  sleep 2
  echo '"LeaveQueue"'
  sleep 1
) | websocat $WS_URL

echo ""
echo "3. Testing with JSON format authentication token"
echo "Connecting to WebSocket..."

# Test 3: Authenticate with JSON token
(
  echo '{"Authenticate":{"token":"{\"user_id\":\"550e8400-e29b-41d4-a716-446655440000\",\"email\":\"bob@example.com\",\"name\":\"Bob\"}"}}'
  sleep 1
  echo '"JoinQueue"'
  sleep 2
) | websocat $WS_URL &

echo ""
echo "4. Testing multiple users for matchmaking"
echo "Connecting second user..."

# Test 4: Second user for matchmaking test
(
  sleep 2
  echo '{"Authenticate":{"token":"user2:charlie@example.com:Charlie"}}'
  sleep 1
  echo '"JoinQueue"'
  sleep 5
) | websocat $WS_URL

wait
echo "Tests completed!"