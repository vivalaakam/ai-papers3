# Backend API Specification

## Overview

This document defines the HTTP/HTTPS REST API for the M5Stack PaperS3 Todo Display application.

**Base URL**: `https://api.example.com/api/v1`

**Authentication**: Bearer token (device token)

**Content-Type**: `application/json`

---

## Authentication

Each device is assigned a unique device token for authentication.

**Header**:
```
Authorization: Bearer <device_token>
```

**Device Token Format**: UUID v4

**Example**:
```
Authorization: Bearer 550e8400-e29b-41d4-a716-446655440000
```

---

## Endpoints

### 1. Get Todos by Date

Fetch all todos for a specific date.

**Endpoint**: `GET /todos/{date}`

**Path Parameters**:
| Name | Type   | Description           | Format      |
|------|--------|-----------------------|-------------|
| date | string | Date to fetch todos   | YYYY-MM-DD  |

**Response Codes**:
| Code | Description             |
|------|-------------------------|
| 200  | Success                 |
| 400  | Invalid date format     |
| 401  | Unauthorized            |
| 404  | No todos for this date  |
| 500  | Server error            |

**Response Body**:
```json
{
  "date": "2026-01-14",
  "todos": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "Buy groceries",
      "description": "Milk, eggs, bread, butter",
      "completed": false,
      "priority": "high",
      "created_at": "2026-01-14T08:00:00Z"
    },
    {
      "id": "660e8400-e29b-41d4-a716-446655440001",
      "title": "Call mom",
      "description": null,
      "completed": true,
      "priority": "medium",
      "created_at": "2026-01-14T09:30:00Z"
    },
    {
      "id": "770e8400-e29b-41d4-a716-446655440002",
      "title": "Review project proposal",
      "description": "Check section 3 and provide feedback",
      "completed": false,
      "priority": "low",
      "created_at": "2026-01-14T10:15:00Z"
    }
  ],
  "updated_at": "2026-01-14T10:30:00Z"
}
```

**Example Request**:
```http
GET /api/v1/todos/2026-01-14 HTTP/1.1
Host: api.example.com
Authorization: Bearer 550e8400-e29b-41d4-a716-446655440000
```

---

### 2. Update Todo Status

Update the completion status of a specific todo.

**Endpoint**: `PATCH /todos/{id}`

**Path Parameters**:
| Name | Type   | Description        |
|------|--------|--------------------|
| id   | string | Todo item ID (UUID) |

**Request Body**:
```json
{
  "completed": true
}
```

**Response Codes**:
| Code | Description           |
|------|-----------------------|
| 200  | Success               |
| 400  | Invalid request       |
| 401  | Unauthorized          |
| 404  | Todo not found        |
| 500  | Server error          |

**Response Body**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "completed": true,
  "updated_at": "2026-01-14T11:00:00Z"
}
```

**Example Request**:
```http
PATCH /api/v1/todos/550e8400-e29b-41d4-a716-446655440000 HTTP/1.1
Host: api.example.com
Authorization: Bearer 550e8400-e29b-41d4-a716-446655440000
Content-Type: application/json

{
  "completed": true
}
```

---

### 3. Device Heartbeat

Send a heartbeat to indicate the device is online and operational.

**Endpoint**: `POST /device/heartbeat`

**Request Body**:
```json
{
  "battery_level": 85,
  "rssi": -45,
  "free_memory": 419430,
  "uptime_seconds": 3600
}
```

**Response Codes**:
| Code | Description       |
|------|-------------------|
| 200  | Success           |
| 401  | Unauthorized      |
| 500  | Server error      |

**Response Body**:
```json
{
  "status": "ok",
  "server_time": "2026-01-14T11:00:00Z",
  "config": {
    "sync_interval_minutes": 60,
    "battery_save_mode": false
  }
}
```

**Example Request**:
```http
POST /api/v1/device/heartbeat HTTP/1.1
Host: api.example.com
Authorization: Bearer 550e8400-e29b-41d4-a716-446655440000
Content-Type: application/json

{
  "battery_level": 85,
  "rssi": -45,
  "free_memory": 419430,
  "uptime_seconds": 3600
}
```

---

### 4. Get Device Configuration

Fetch device-specific configuration from the server.

**Endpoint**: `GET /device/config`

**Response Codes**:
| Code | Description       |
|------|-------------------|
| 200  | Success           |
| 401  | Unauthorized      |
| 500  | Server error      |

**Response Body**:
```json
{
  "device_id": "550e8400-e29b-41d4-a716-446655440000",
  "sync_interval_minutes": 60,
  "battery_save_mode": false,
  "auto_refresh_hours": [6, 12, 18],
  "timezone": "Asia/Shanghai",
  "display_config": {
    "show_completed": true,
    "show_priority": true,
    "max_display_items": 10
  }
}
```

**Example Request**:
```http
GET /api/v1/device/config HTTP/1.1
Host: api.example.com
Authorization: Bearer 550e8400-e29b-41d4-a716-446655440000
```

---

## Error Responses

All error responses follow this format:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "details": {}
  }
}
```

**Common Error Codes**:

| Code    | Description                      |
|---------|----------------------------------|
| INVALID_REQUEST | Malformed request body      |
| UNAUTHORIZED    | Missing or invalid token     |
| FORBIDDEN       | Insufficient permissions     |
| NOT_FOUND       | Resource not found           |
| RATE_LIMITED    | Too many requests            |
| SERVER_ERROR    | Internal server error        |

**Example Error Response**:
```json
{
  "error": {
    "code": "INVALID_DATE_FORMAT",
    "message": "Date must be in YYYY-MM-DD format",
    "details": {
      "received": "2026/01/14",
      "expected_format": "YYYY-MM-DD"
    }
  }
}
```

---

## Data Types

### TodoPriority Enum

| Value   | Description |
|---------|-------------|
| low     | Low priority |
| medium  | Medium priority |
| high    | High priority |

---

## Rate Limiting

- **Default Limit**: 100 requests per hour per device
- **Burst Limit**: 10 requests per minute

**Rate Limit Headers**:
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1705228800
```

---

## HTTP Status Codes

| Code | Description           |
|------|-----------------------|
| 200  | OK                    |
| 201  | Created               |
| 204  | No Content            |
| 400  | Bad Request           |
| 401  | Unauthorized          |
| 403  | Forbidden             |
| 404  | Not Found             |
| 429  | Too Many Requests     |
| 500  | Internal Server Error |
| 503  | Service Unavailable   |

---

## Device Provisioning Flow

1. **Device Registration** (first-time setup)
   - Device sends registration request with serial number
   - Server responds with device token

2. **Token Storage**
   - Device stores token in NVS
   - Token used for all subsequent requests

3. **Token Refresh**
   - Tokens expire after 1 year
   - Device can request refresh before expiration

---

## Sync Strategy

### Recommended Sync Pattern

1. **On Boot**: Fetch today's todos immediately
2. **Periodic**: Sync every 60 minutes (configurable)
3. **On User Action**: Sync when user presses sync button
4. **On Resume**: Sync after waking from sleep

### Offline Behavior

- Display last successfully fetched todos
- Queue todo status updates locally
- Apply queued updates when connection restored

### Battery Optimization

- Increase sync interval when battery < 20%
- Skip sync if battery < 5%
- Use WiFi power saving mode

---

## Security Considerations

1. **HTTPS Only**: All requests must use HTTPS in production
2. **Token Storage**: Device tokens must be stored securely in NVS
3. **Certificate Validation**: Validate server certificates
4. **Token Expiration**: Handle token expiration gracefully
5. **Device Binding**: Tokens are bound to specific device IDs

---

## Testing Endpoints

For local development, use these mock endpoints:

**Base URL**: `http://localhost:8080/api/v1`

**Mock Credentials**:
```
Device Token: test-device-token-123
```

**Mock Date**: Always returns data for any valid date format
