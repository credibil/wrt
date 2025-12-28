buildgen::guest!({
    http: [
        "/jobs/detector": {
            method: get,
            request: String
            handler: DetectionRequest 
            response: DetectionResponse
        }
    ],
    messaging: [
        "realtime-r9k.v1": {
            message: R9kMessage
        }
    ]
});