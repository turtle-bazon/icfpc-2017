(define-module (lib replay)
  #:use-module (ice-9 format)
  #:use-module (ice-9 match)
  #:use-module (ice-9 rdelim)
  #:use-module (json)
  #:use-module (lib game)
  #:use-module (lib game-data)
  #:use-module (lib log)
  #:use-module (srfi srfi-9)
  #:export (generate-replay))

(define _SERVER_TO_PUNTER_ "S->P")

(define _PUNTER_TO_SERVER_ "P->S")

(define-record-type replay-context
  (make-replay-context game-map game-state)
  replay-context?
  (game-map rc-game-map set-rc-game-map!)
  (game-state rc-game-state set-rc-game-state!))

(define (handle-p->s rctx jsval)
  #nil)

(define (handle-s->p rctx jsval)
  (flog-msg 'DEBUG "JSON: ~a~&" jsval)
  #nil)

(define (handle-protocol-message rctx data-type data)
  (flog-msg 'DEBUG data)
  (let* ((colon-sep (string-index data #\:))
         (msg-len (string->number (substring data 0 colon-sep)))
         (msg (substring data (+ colon-sep 1)))
         (json-data (json-string->scm msg)))
    (match data-type
      (_SERVER_TO_PUNTER_ (handle-s->p rctx json-data))
      (_PUNTER_TO_SERVER_ (handle-p->s rctx json-data)))))

(define (handle-unknown-message rctx data)
  ;; ignore now
  #nil)

(define (handle-line rctx line)
  (let ((info-sep (string-index line #\|)))
    (when info-sep
      (let ((info-type (string-trim-both (substring line 0 info-sep)))
            (info-data (string-trim-both (substring line (+ info-sep 1)))))
        (match info-type
          ((_SERVER_TO_PUNTER_
            _PUNTER_TO_SERVER_) (handle-protocol-message rctx info-type info-data))
          (_ (handle-unknown-message rctx info-data)))))))

(define (generate-replay replay-file output-dir)
  (let ((rctx (make-replay-context #nil #nil)))
    (with-input-from-file replay-file
      (lambda ()
        (let loop ((line (read-line)))
          (if (not (eof-object? line))
              (when (not (string=? "" line))
                (handle-line rctx line)
                (loop (read-line)))))))))
