buildgen::guest!({
    owner: "at",
    provider: MyProvider,
    http: [
        "/jobs/detector": {
            method: get,
            request: DetectionRequest
            reply: DetectionReply
        },
        "/god-mode/set-trip/{vehicle_id}/{trip_id}": {
            method: get,
            request: SetTripRequest
            reply: SetTripReply
        }
    ],
    messaging: [
        "realtime-r9k.v1": {
            message: R9kMessage
        }
    ]
});
