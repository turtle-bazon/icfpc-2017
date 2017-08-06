(define-module (lib loader)
  #:use-module (json)
  #:use-module (lib game-data)
  #:export (transform->game-map
            load-game-map))

(define (transform->site site)
  (make-site (hash-ref site "id")
             (hash-ref site "x")
             (hash-ref site "y")))

(define (transform->river river)
  (make-river (hash-ref river "source")
              (hash-ref river "target")))

(define (transform->sites sites-list)
  (map transform->site sites-list))

(define (transform->mines mines-list)
  mines-list)

(define (transform->rivers rivers-list)
  (map transform->river rivers-list))

(define (transform->game-map hashtable)
  (make-game-map
   (transform->sites (hash-ref hashtable "sites"))
   (transform->mines (hash-ref hashtable "mines"))
   (transform->rivers (hash-ref hashtable "rivers"))))

(define (load-game-map file)
  (transform->game-map
   (with-input-from-file file
     (lambda ()
       (json->scm (current-input-port))))))
